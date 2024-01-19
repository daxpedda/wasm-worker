//! Implementation without the atomics target feature enabled.

use std::cell::OnceCell;
use std::fmt::{self, Debug, Formatter};
use std::io::{self, Error, ErrorKind};
use std::marker::PhantomData;
use std::sync::Arc;
use std::thread::Result;
use std::time::Duration;

use js_sys::{Atomics, Int32Array, SharedArrayBuffer};

use super::global::{Global, GLOBAL};
use super::{Scope, ScopedJoinHandle, ThreadId};
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
		Err(Error::new(
			ErrorKind::Unsupported,
			"operation not supported on this platform without the atomics target feature",
		))
	}

	/// Implementation of [`std::thread::Builder::spawn_scoped()`].
	#[allow(clippy::unused_self)]
	pub(super) fn spawn_scoped<'scope, F, T>(
		self,
		_: &'scope Scope<'scope, '_>,
		_: F,
	) -> io::Result<ScopedJoinHandle<'scope, T>> {
		Err(Error::new(
			ErrorKind::Unsupported,
			"operation not supported on this platform without the atomics target feature",
		))
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

/// Implementation of [`std::thread::Thread`].
#[derive(Clone, Debug)]
pub(super) struct Thread(Arc<ThreadInner>);

/// Inner shared wrapper for [`Thread`].
#[derive(Debug)]
struct ThreadInner {
	/// [`ThreadId`].
	id: ThreadId,
	/// Name of the thread.
	name: Option<String>,
}

impl Thread {
	/// Create a new [`Thread`].
	fn new() -> Self {
		let name = GLOBAL.with(|global| match global.as_ref()? {
			Global::Worker(worker) => Some(worker.name()),
			Global::Window(_) | Global::Worklet => None,
		});

		Self(Arc::new(ThreadInner {
			id: ThreadId::new(),
			name,
		}))
	}

	/// Gets the current [`Thread`] and instantiates it if not set.
	pub(super) fn current() -> Self {
		thread_local! {
			/// Holds this threads [`Thread`].
			static THREAD: OnceCell<Thread> = OnceCell::new();
		}

		THREAD.with(|cell| cell.get_or_init(Self::new).clone())
	}

	/// Implementation of [`std::thread::Thread::id()`].
	pub(super) fn id(&self) -> ThreadId {
		self.0.id
	}

	/// Implementation of [`std::thread::Thread::name()`].
	pub(super) fn name(&self) -> Option<&str> {
		self.0.name.as_deref()
	}

	/// Implementation of [`std::thread::Thread::unpark()`].
	#[allow(clippy::missing_const_for_fn, clippy::unused_self)]
	pub(super) fn unpark(&self) {}
}

/// Implementation of [`std::thread::park()`].
#[allow(clippy::missing_const_for_fn)]
pub(super) fn park() {}

/// Implementation of [`std::thread::park_timeout()`].
#[allow(clippy::missing_const_for_fn)]
pub(super) fn park_timeout(_: Duration) {}

/// Implementation of [`std::thread::park_timeout_ms()`].
#[allow(clippy::missing_const_for_fn)]
pub(super) fn park_timeout_ms(_: u32) {}

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
	let buffer = SharedArrayBuffer::new(1);
	let buffer = Int32Array::new(&buffer);
	#[allow(clippy::as_conversions, clippy::cast_precision_loss)]
	let timeout = dur.as_millis() as f64;
	let val = Atomics::wait_with_timeout(&buffer, 0, 0, timeout)
		.expect("current thread cannot be blocked");
	debug_assert_eq!(
		val, "timed-out",
		"unexpected return value from `Atomics.wait"
	);
}

/// Implementation of [`std::thread::sleep_ms()`].
pub(super) fn sleep_ms(ms: u32) {
	sleep(Duration::from_millis(ms.into()));
}
