//! Platform-specific extensions to `web-thread` for the Web platform.

use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project::pin_project;

use crate::{thread, JoinHandle, Scope, ScopedJoinHandle};

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
	type Output = crate::Result<T>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		JoinHandle::poll(self.0, cx)
	}
}

/// Async version of [`scope()`](std::thread::scope).
pub fn scope_async<'scope, 'env: 'scope, F1, F2, T>(
	#[allow(clippy::min_ident_chars)] f: F1,
) -> ScopeFuture<'scope, 'env, F2, T>
where
	F1: FnOnce(&'scope Scope<'scope, 'env>) -> F2,
	F2: Future<Output = T>,
{
	ScopeFuture(thread::scope_async(f))
}

/// Waits for the associated scope to finish.
///
/// If dropped but not polled to completion, will block until all spawned
/// threads are finished but does not guarantee that the passed [`Future`] has
/// finished executing.
#[must_use = "does nothing if not polled"]
#[pin_project]
pub struct ScopeFuture<'scope, 'env, F2, T>(#[pin] thread::ScopeFuture<'scope, 'env, F2, T>);

impl<F2, T> Debug for ScopeFuture<'_, '_, F2, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.debug_tuple("ScopeFuture").field(&self.0).finish()
	}
}

impl<F2, T> Future for ScopeFuture<'_, '_, F2, T>
where
	F2: Future<Output = T>,
{
	type Output = T;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self.project().0.poll(cx)
	}
}

/// Web-specific extension to
/// [`web_thread::ScopedJoinHandle`](crate::ScopedJoinHandle).
pub trait ScopedJoinHandleExt<'scope, T> {
	/// Async version of [`ScopedJoinHandle::join()`].
	fn join_async<'handle>(&'handle mut self) -> ScopedJoinHandleFuture<'handle, 'scope, T>;
}

impl<'scope, T> ScopedJoinHandleExt<'scope, T> for ScopedJoinHandle<'scope, T> {
	fn join_async<'handle>(&'handle mut self) -> ScopedJoinHandleFuture<'handle, 'scope, T> {
		ScopedJoinHandleFuture(self)
	}
}

/// Waits for the associated thread to finish.
#[must_use = "does nothing if not polled"]
pub struct ScopedJoinHandleFuture<'handle, 'scope, T>(&'handle mut ScopedJoinHandle<'scope, T>);

impl<T> Debug for ScopedJoinHandleFuture<'_, '_, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_tuple("JoinHandleFuture")
			.field(&self.0)
			.finish()
	}
}

impl<T> Future for ScopedJoinHandleFuture<'_, '_, T> {
	type Output = crate::Result<T>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		ScopedJoinHandle::poll(self.0, cx)
	}
}
