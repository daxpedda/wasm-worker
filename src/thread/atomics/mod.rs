//! Implementation when the atomics target feature is enabled.

mod channel;
mod js;
mod parker;
mod spawn;
mod url;
mod wait_async;

use std::fmt::{self, Debug, Formatter};
use std::io;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock, PoisonError, TryLockError};
use std::task::{Context, Poll};
use std::thread::Result;
use std::time::Duration;

use atomic_waker::AtomicWaker;
use js_sys::WebAssembly::{Memory, Module};
use js_sys::{Atomics, Int32Array};
use wasm_bindgen::JsCast;

pub(super) use self::parker::Parker;
use super::js::{GlobalExt, CROSS_ORIGIN_ISOLATED};
use super::{ScopedJoinHandle, Thread, THREAD};
use crate::thread;

/// Implementation of [`std::thread::Builder`].
#[derive(Debug)]
pub(super) struct Builder {
	/// Name of the thread.
	name: Option<String>,
}

impl Builder {
	/// Implementation of [`std::thread::Builder::new()`].
	#[allow(clippy::missing_const_for_fn, clippy::new_without_default)]
	pub(super) fn new() -> Self {
		Self { name: None }
	}

	/// Implementation of [`std::thread::Builder::name()`].
	pub(super) fn name(mut self, name: String) -> Self {
		self.name = Some(name);
		self
	}

	/// Implementation of [`std::thread::Builder::spawn()`].
	pub(super) fn spawn<F, T>(self, task: F) -> io::Result<JoinHandle<T>>
	where
		F: 'static + FnOnce() -> T + Send,
		T: Send + 'static,
	{
		// SAFETY: `F` and `T` are `'static`.
		unsafe { spawn::spawn(task, self.name, None) }
	}

	/// Implementation of [`std::thread::Builder::spawn_scoped()`].
	pub(super) fn spawn_scoped<'scope, F, T>(
		self,
		scope: &'scope Scope,
		task: F,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F: FnOnce() -> T + Send + 'scope,
		T: Send + 'scope,
	{
		// SAFETY: `Scope` will prevent this thread to outliving its lifetime.
		let result = unsafe { spawn::spawn(task, self.name, Some(Arc::clone(&scope.0))) };

		result.map(|handle| ScopedJoinHandle {
			handle,
			_scope: PhantomData,
		})
	}
}

/// Implementation of [`std::thread::JoinHandle`].
pub(super) struct JoinHandle<T> {
	/// Shared state between [`JoinHandle`] and thread.
	shared: Arc<spawn::Shared<T>>,
	/// Corresponding [`Thread`].
	thread: Thread,
	/// Marker to know if the return value was already taken by
	/// [`JoinHandle::join_async()`](crate::web::JoinHandleExt::join_async).
	taken: AtomicBool,
}

impl<T> Debug for JoinHandle<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("JoinHandle")
			.field("shared", &self.shared)
			.field("thread", &self.thread)
			.field("taken", &self.taken)
			.finish()
	}
}

impl<T> JoinHandle<T> {
	/// Implementation of [`std::thread::JoinHandle::is_finished()`].
	pub(super) fn is_finished(&self) -> bool {
		if self.taken.load(Ordering::Relaxed) {
			return true;
		}

		#[allow(clippy::significant_drop_in_scrutinee)]
		match self.shared.value.try_lock().as_deref() {
			Ok(Some(_)) => true,
			Err(TryLockError::Poisoned(error)) => error.get_ref().is_some(),
			Err(TryLockError::WouldBlock) | Ok(None) => false,
		}
	}

	/// Implementation of [`std::thread::JoinHandle::join()`].
	#[allow(clippy::unnecessary_wraps)]
	pub(super) fn join(self) -> Result<T> {
		let mut value = self
			.shared
			.value
			.lock()
			.unwrap_or_else(PoisonError::into_inner);

		while value.is_none() {
			value = self
				.shared
				.cvar
				.wait(value)
				.unwrap_or_else(PoisonError::into_inner);
		}

		Ok(value.take().expect("no value found after notification"))
	}

	/// Implementation of [`std::thread::JoinHandle::thread()`].
	#[allow(clippy::missing_const_for_fn)]
	pub(super) fn thread(&self) -> &Thread {
		&self.thread
	}

	/// Implementation for
	/// [`JoinHandleFuture::poll()`](crate::web::JoinHandleFuture).
	#[allow(clippy::needless_pass_by_ref_mut)]
	pub(super) fn poll(&self, cx: &Context<'_>) -> Poll<Result<T>> {
		assert!(
			!self.taken.load(Ordering::Relaxed),
			"`JoinHandleFuture` polled or created after completion"
		);

		let mut value = match self.shared.value.try_lock() {
			Ok(mut value) => value.take(),
			Err(TryLockError::Poisoned(error)) => error.into_inner().take(),
			Err(TryLockError::WouldBlock) => None,
		};

		if value.is_none() {
			self.shared.waker.register(cx.waker());

			value = match self.shared.value.try_lock() {
				Ok(mut value) => value.take(),
				Err(TryLockError::Poisoned(error)) => error.into_inner().take(),
				Err(TryLockError::WouldBlock) => None,
			};
		}

		if let Some(value) = value {
			self.taken.store(true, Ordering::Relaxed);
			Poll::Ready(Ok(value))
		} else {
			Poll::Pending
		}
	}
}

impl Thread {
	/// Registers the given `thread`.
	fn register(thread: Self) {
		THREAD.with(|cell| cell.set(thread).expect("`Thread` already registered"));
	}
}

/// Implementation of [`std::thread::Scope`].
#[derive(Debug)]
pub(super) struct Scope(Arc<ScopeData>);

/// Shared data between [`Scope`] and scoped threads.
#[derive(Debug)]
pub(super) struct ScopeData {
	/// Number of running threads.
	threads: AtomicUsize,
	/// Handle to the spawning thread.
	thread: Thread,
	/// [`Waker`](std::task::Waker) to wake up a waiting [`Scope`].
	waker: AtomicWaker,
}

impl Scope {
	/// Creates a new [`Scope`].
	pub(super) fn new() -> Self {
		Self(Arc::new(ScopeData {
			threads: AtomicUsize::new(0),
			thread: thread::current(),
			waker: AtomicWaker::new(),
		}))
	}

	/// End the scope after calling the user function.
	pub(super) fn finish(&self) {
		while self.0.threads.load(Ordering::Acquire) != 0 {
			thread::park();
		}
	}

	/// End the scope after calling the user function.
	pub(super) fn finish_async(&self, cx: &Context<'_>) -> Poll<()> {
		if self.0.threads.load(Ordering::Acquire) == 0 {
			return Poll::Ready(());
		}

		self.0.waker.register(cx.waker());

		if self.0.threads.load(Ordering::Acquire) == 0 {
			Poll::Ready(())
		} else {
			Poll::Pending
		}
	}
}

/// Implementation of [`std::thread::sleep()`].
pub(super) fn sleep(dur: Duration) {
	#[allow(clippy::absolute_paths)]
	std::thread::sleep(dur);
}

/// Tests is waiting is supported.
pub(super) fn test_wait_support() -> bool {
	let value = 0;
	let index: *const i32 = &value;
	#[allow(clippy::as_conversions)]
	let index = index as u32 / 4;

	MEMORY_ARRAY
		.with(|array| Atomics::wait_with_timeout(array, index, 0, 0.))
		.is_ok()
}

/// Implementation for [`crate::web::has_spawn_support()`]. Make sure to
/// instantiate it on the main thread!
pub(super) fn has_spawn_support() -> bool {
	/// We spawn only from the main thread, so we cache the result to be able to
	/// call it from other threads but get the result of the main thread.
	static HAS_SPAWN_SUPPORT: OnceLock<bool> = OnceLock::new();

	*HAS_SPAWN_SUPPORT.get_or_init(|| {
		*CROSS_ORIGIN_ISOLATED && {
			let global: GlobalExt = js_sys::global().unchecked_into();
			!global.worker().is_undefined()
		}
	})
}

thread_local! {
	/// [`Memory`] of the Wasm module.
	pub(super) static MEMORY: Memory = wasm_bindgen::memory().unchecked_into();
	/// [`Memory`] of the Wasm module as a [`Int32Array`].
	pub(super) static MEMORY_ARRAY: Int32Array = Int32Array::new(&MEMORY.with(Memory::buffer));
	/// Wasm [`Module`].
	pub(super) static MODULE: Module = wasm_bindgen::module().unchecked_into();
}
