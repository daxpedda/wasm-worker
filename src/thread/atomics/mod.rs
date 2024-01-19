//! Implementation when the atomics target feature is enabled.

mod channel;
mod js;
mod parker;
mod spawn;
mod url;
mod wait_async;

use std::fmt::{self, Debug, Formatter};
use std::io;
use std::sync::{Arc, PoisonError};
use std::thread::Result;
use std::time::Duration;

pub(super) use self::parker::Parker;
use super::{Scope, ScopedJoinHandle, Thread, THREAD};

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
		spawn::spawn(task, self.name)
	}

	/// Implementation of [`std::thread::Builder::spawn_scoped()`].
	pub(super) fn spawn_scoped<'scope, F, T>(
		self,
		_scope: &'scope Scope<'scope, '_>,
		_task: F,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F: FnOnce() -> T + Send + 'scope,
		T: Send + 'scope,
	{
		todo!()
	}
}

/// Implementation of [`std::thread::JoinHandle`].
pub(crate) struct JoinHandle<T> {
	/// Shared state between [`JoinHandle`] and thread.
	pub(crate) shared: Arc<spawn::Shared<T>>,
	/// Corresponding [`Thread`].
	pub(crate) thread: Thread,
}

impl<T> Debug for JoinHandle<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("JoinHandle")
			.field("shared", &self.shared)
			.field("thread", &self.thread)
			.finish()
	}
}

impl<T> JoinHandle<T> {
	/// Implementation of [`std::thread::JoinHandle::is_finished()`].
	pub(super) fn is_finished(&self) -> bool {
		Arc::strong_count(&self.shared) == 1
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
}

impl Thread {
	/// Registers the given `thread`.
	fn register(thread: Self) {
		THREAD.with(|cell| cell.set(thread).expect("`Thread` already registered"));
	}
}

/// Implementation of [`std::thread::scope()`].
#[track_caller]
pub(super) fn scope<'env, F, T>(_task: F) -> T
where
	F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T,
{
	todo!()
}

/// Implementation of [`std::thread::sleep()`].
pub(super) fn sleep(dur: Duration) {
	#[allow(clippy::absolute_paths)]
	std::thread::sleep(dur);
}

/// Implementation of [`std::thread::sleep_ms()`].
pub(super) fn sleep_ms(ms: u32) {
	#[allow(clippy::absolute_paths, deprecated)]
	std::thread::sleep_ms(ms);
}
