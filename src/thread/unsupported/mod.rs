//! Implementation without the atomics target feature enabled.

mod parker;

use std::fmt::{self, Debug, Formatter};
use std::io;
use std::marker::PhantomData;
use std::thread::Result;
use std::time::Duration;

use js_sys::{Atomics, Int32Array, SharedArrayBuffer};

pub(super) use self::parker::Parker;
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
		.with(|array| Atomics::wait_with_timeout(array, 0, 0, timeout))
		.expect("`Atomics.wait` is not expected to fail");
	debug_assert_eq!(
		result, "timed-out",
		"unexpected return value from `Atomics.wait"
	);
}

thread_local! {
	static ZERO_ARRAY: Int32Array = Int32Array::new(&SharedArrayBuffer::new(4));
}
