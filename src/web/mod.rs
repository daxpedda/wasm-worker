//! Platform-specific extensions to `web-thread` for the Web platform.

use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::thread::{self, JoinHandle};

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
			use std::sync::TryLockError;

			let value = match self.0.shared.value.try_lock() {
				Ok(mut value) => value.take(),
				Err(TryLockError::Poisoned(error)) => error.into_inner().take(),
				Err(TryLockError::WouldBlock) => None,
			};

			if let Some(value) = value {
				return Poll::Ready(Ok(value));
			}

			self.0.shared.waker.register(cx.waker());

			let value = match self.0.shared.value.try_lock() {
				Ok(mut value) => value.take(),
				Err(TryLockError::Poisoned(error)) => error.into_inner().take(),
				Err(TryLockError::WouldBlock) => None,
			};

			if let Some(value) = value {
				Poll::Ready(Ok(value))
			} else {
				Poll::Pending
			}
		}
		#[cfg(not(target_feature = "atomics"))]
		unreachable!("found instanced `JoinHandle` without threading support")
	}
}
