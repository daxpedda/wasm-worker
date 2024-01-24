//! Platform-specific extensions to `web-thread` for the Web platform.

use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{self as thread, JoinHandle};

/// Returns [`true`] if the current thread supports waiting, e.g. parking and
/// sleeping.
#[must_use]
pub fn has_wait_support() -> bool {
	thread::has_wait_support()
}

/// Returns [`true`] if the platform supports spawning threads.
#[must_use]
pub fn has_spawn_support() -> bool {
	thread::has_spawn_support()
}

/// Web-specific extension to [`web_thread::JoinHandle`](crate::JoinHandle).
pub trait JoinHandleExt<T> {
	/// Async version of [`JoinHandle::join()`].
	fn join_async(&mut self) -> JoinHandleFuture<'_, T>;
}

impl<T> JoinHandleExt<T> for JoinHandle<T> {
	fn join_async(&mut self) -> JoinHandleFuture<'_, T> {
		JoinHandleFuture(self)
	}
}

/// Waits for the associated thread to finish.
#[must_use = "does nothing if not polled"]
pub struct JoinHandleFuture<'handle, T>(&'handle mut JoinHandle<T>);

impl<T> Debug for JoinHandleFuture<'_, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_tuple("JoinHandleFuture")
			.field(&self.0)
			.finish()
	}
}

impl<T> Future for JoinHandleFuture<'_, T> {
	type Output = thread::Result<T>;

	fn poll(
		self: Pin<&mut Self>,
		#[cfg_attr(not(target_feature = "atomics"), allow(unused_variables))] cx: &mut Context<'_>,
	) -> Poll<Self::Output> {
		#[cfg(target_feature = "atomics")]
		{
			use std::sync::atomic::Ordering;
			use std::sync::TryLockError;

			assert!(
				!self.0 .0.taken.load(Ordering::Relaxed),
				"`JoinHandleFuture` polled or created after completion"
			);

			let mut value = match self.0 .0.shared.value.try_lock() {
				Ok(mut value) => value.take(),
				Err(TryLockError::Poisoned(error)) => error.into_inner().take(),
				Err(TryLockError::WouldBlock) => None,
			};

			if value.is_none() {
				self.0 .0.shared.waker.register(cx.waker());

				value = match self.0 .0.shared.value.try_lock() {
					Ok(mut value) => value.take(),
					Err(TryLockError::Poisoned(error)) => error.into_inner().take(),
					Err(TryLockError::WouldBlock) => None,
				};
			}

			if let Some(value) = value {
				self.0 .0.taken.store(true, Ordering::Relaxed);
				Poll::Ready(Ok(value))
			} else {
				Poll::Pending
			}
		}
		#[cfg(not(target_feature = "atomics"))]
		unreachable!("found instanced `JoinHandle` without threading support")
	}
}
