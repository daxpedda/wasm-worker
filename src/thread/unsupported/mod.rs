//! Implementation without the atomics target feature enabled.

mod js;
mod parker;

use std::fmt::{self, Debug, Formatter};
use std::io;
use std::marker::PhantomData;
use std::thread::Result;
use std::time::Duration;

use js::MemoryDescriptor;
use js_sys::WebAssembly::Memory;
use js_sys::{Atomics, Int32Array, Object, SharedArrayBuffer};
use wasm_bindgen::JsCast;

pub(super) use self::parker::Parker;
use super::global::{Global, GLOBAL};
use super::{Scope, ScopedJoinHandle};
use crate::thread;

/// Implementation of [`std::thread::Builder`].
#[derive(Debug)]
pub(super) struct Builder;

impl Builder {
	/// Implementation of [`std::thread::Builder::new()`].
	#[allow(clippy::missing_const_for_fn)]
	pub(super) fn new() -> Self {
		Self
	}

	/// Implementation of [`std::thread::Builder::name()`].
	pub(super) fn name(self, _: String) -> Self {
		self
	}

	/// Implementation of [`std::thread::Builder::spawn()`].
	#[allow(clippy::unused_self)]
	pub(super) fn spawn<F, T>(self, _: F) -> io::Result<JoinHandle<T>> {
		unreachable!("reached `spawn()` without atomics target feature")
	}

	/// Implementation of [`std::thread::Builder::spawn_scoped()`].
	#[allow(clippy::unused_self)]
	pub(super) fn spawn_scoped<'scope, F, T>(
		self,
		_: &'scope Scope<'scope, '_>,
		_: F,
	) -> io::Result<ScopedJoinHandle<'scope, T>> {
		unreachable!("reached `spawn_scoped()` without atomics target feature")
	}
}

/// Implementation of [`std::thread::JoinHandle`].
pub(crate) struct JoinHandle<T>(PhantomData<T>);

impl<T> Debug for JoinHandle<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.debug_tuple("JoinHandle").finish()
	}
}

impl<T> JoinHandle<T> {
	/// Implementation of [`std::thread::JoinHandle::is_finished()`].
	#[allow(clippy::unused_self)]
	pub(super) fn is_finished(&self) -> bool {
		unreachable!("found instanced `JoinHandle` without threading support")
	}

	/// Implementation of [`std::thread::JoinHandle::join()`].
	#[allow(clippy::unused_self)]
	pub(super) fn join(self) -> Result<T> {
		unreachable!("found instanced `JoinHandle` without threading support")
	}

	/// Implementation of [`std::thread::JoinHandle::thread()`].
	#[allow(clippy::unused_self)]
	pub(super) fn thread(&self) -> &thread::Thread {
		unreachable!("found instanced `JoinHandle` without threading support")
	}
}

/// Implementation of [`std::thread::scope()`].
#[track_caller]
pub(super) fn scope<'env, F, T>(_: F) -> T
where
	F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T,
{
	todo!()
}

/// Implementation of [`std::thread::sleep()`].
pub(super) fn sleep(dur: Duration) {
	#[allow(clippy::as_conversions, clippy::cast_precision_loss)]
	let timeout = dur.as_millis() as f64;
	let result = ZERO_ARRAY
		.with(|array| {
			let Some(array) = array else {
				unreachable!("forgot to check wait support first");
			};
			Atomics::wait_with_timeout(array, 0, 0, timeout)
		})
		.expect("`Atomics.wait` is not expected to fail");
	debug_assert_eq!(
		result, "timed-out",
		"unexpected return value from `Atomics.wait"
	);
}

/// Determines if a shared worker has wait support.
pub(super) fn has_shared_worker_wait_support() -> bool {
	thread_local! {
		static HAS_SHARED_WORKER_WAIT_SUPPORT: bool = ZERO_ARRAY.with(|array| {
			let Some(array) = array else { return false };
			Atomics::wait_with_timeout(array, 0, 0, 0.).is_ok()
		});
	}

	debug_assert!(
		GLOBAL.with(|global| matches!(global, Some(Global::Shared(_)))),
		"called `has_shared_worker_wait_support` outside a shared worker"
	);

	HAS_SHARED_WORKER_WAIT_SUPPORT.with(bool::clone)
}

/// Implementation for [`crate::web::has_spawn_support()`].
pub(crate) fn has_spawn_support() -> bool {
	false
}

thread_local! {
	static ZERO_ARRAY: Option<Int32Array> = {
		GLOBAL.with(|global| {
			if matches!(global, Some(Global::Shared(_))) {
				// Shared workers currently don't support `new SharedArrayBuffer`, but they
				// still support Wasm's shared memory, which is a `SharedArrayBuffer` underneath.
				let descriptor: MemoryDescriptor = Object::new().unchecked_into();
				descriptor.set_initial(1);
				descriptor.set_maximum(1);
				descriptor.set_shared(true);
				let Ok(memory) = Memory::new(&descriptor) else {
					return None;
				};
				Some(Int32Array::new(&memory.buffer()))
			} else {
				Some(Int32Array::new(&SharedArrayBuffer::new(4)))
			}
		})
	};
}
