//! Platform-specific extensions to `web-thread` for the Web platform.

use std::fmt::{self, Debug, Formatter};
use std::future::{Future, Ready};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
mod thread {
	pub(super) struct ScopeFuture<'scope, 'env, F, T>(&'scope &'env (F, T));
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use pin_project::pin_project;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use crate::thread;
use crate::{Builder, JoinHandle, Scope, ScopedJoinHandle};

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

/// Waits for the associated thread to finish. See
/// [`JoinHandleExt::join_async()`].
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

/// Waits for the associated scope to finish. See [`scope_async()`].
///
/// If dropped but not polled to completion, will block until all spawned
/// threads are finished but does not guarantee that the passed [`Future`] has
/// finished executing.
#[must_use = "will block until all spawned threads are finished if not polled to completion"]
#[cfg_attr(all(target_family = "wasm", target_os = "unknown"), pin_project)]
pub struct ScopeFuture<'scope, 'env, F, T>(
	#[cfg_attr(all(target_family = "wasm", target_os = "unknown"), pin)]
	thread::ScopeFuture<'scope, 'env, F, T>,
);

impl<F, T> Debug for ScopeFuture<'_, '_, F, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.debug_tuple("ScopeFuture").field(&self.0).finish()
	}
}

impl<F, T> Future for ScopeFuture<'_, '_, F, T>
where
	F: Future<Output = T>,
{
	type Output = T;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self.project().0.poll(cx)
	}
}

impl<'scope, 'env, F, T> ScopeFuture<'scope, 'env, F, T> {
	/// Converts this [`ScopeFuture`] to a [`ScopeWaitFuture`] by waiting until
	/// the given [`Future`] to [`scope_async()`] is finished.
	///
	/// This is useful to get rid of `F` which often prevents [`ScopeFuture`] to
	/// implement [`Unpin`].
	pub const fn into_wait(self) -> ScopeIntoWaitFuture<'scope, 'env, F, T> {
		ScopeIntoWaitFuture(self)
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

/// Waits for the associated thread to finish. See
/// [`ScopedJoinHandleExt::join_async()`].
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

/// Poll to completion to get a [`ScopeWaitFuture`]. See
/// [`ScopeFuture::into_wait()`].
///
/// If dropped but not polled to completion, will block until all spawned
/// threads are finished but does not guarantee that the passed [`Future`] has
/// finished executing.
#[must_use = "will block until all spawned threads are finished if not polled to completion"]
#[cfg_attr(all(target_family = "wasm", target_os = "unknown"), pin_project)]
pub struct ScopeIntoWaitFuture<'scope, 'env, F, T>(
	#[cfg_attr(all(target_family = "wasm", target_os = "unknown"), pin)]
	ScopeFuture<'scope, 'env, F, T>,
);

impl<F, T> Debug for ScopeIntoWaitFuture<'_, '_, F, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_tuple("ScopeIntoWaitFuture")
			.field(&self.0)
			.finish()
	}
}

impl<'scope, 'env, F, T> Future for ScopeIntoWaitFuture<'scope, 'env, F, T>
where
	F: Future<Output = T>,
{
	type Output = ScopeWaitFuture<'scope, 'env, T>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self.project()
			.0
			.project()
			.0
			.poll_into_wait(cx)
			.map(ScopeFuture)
			.map(ScopeWaitFuture)
	}
}

impl<'scope, 'env, F, T> ScopeIntoWaitFuture<'scope, 'env, F, T> {
	/// Reverts back to [`ScopeFuture`]. See [`ScopeFuture::into_wait()`].
	pub fn revert(self) -> ScopeFuture<'scope, 'env, F, T> {
		self.0
	}
}

/// Waits for the associated scope to finish. See [`ScopeFuture::into_wait()`].
///
/// If dropped but not polled to completion, will block until all spawned
/// threads are finished.
#[must_use = "will block until all spawned threads are finished if not polled to completion"]
pub struct ScopeWaitFuture<'scope, 'env, T>(ScopeFuture<'scope, 'env, Ready<T>, T>);

impl<T> Debug for ScopeWaitFuture<'_, '_, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_tuple("ScopeWaitFuture")
			.field(&self.0)
			.finish()
	}
}

impl<T> Future for ScopeWaitFuture<'_, '_, T> {
	type Output = T;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Pin::new(&mut self.0).poll(cx)
	}
}

impl<T> ScopeWaitFuture<'_, '_, T> {
	/// This will block until all associated threads are finished.
	///
	/// # Panics
	/// If called after being polled to completion.
	pub fn join(self) -> T {
		self.0 .0.join()
	}
}

/// Web-specific extension to [`web_thread::Builder`](crate::Builder).
pub trait BuilderExt {
	/// Async version of [`Builder::spawn()`].
	#[allow(clippy::missing_errors_doc)]
	fn spawn_async<F1, F2, T>(
		self,
		#[allow(clippy::min_ident_chars)] f: F1,
	) -> io::Result<JoinHandle<T>>
	where
		F1: 'static + FnOnce() -> F2 + Send,
		F2: 'static + Future<Output = T>,
		T: 'static + Send;

	/// Async version of [`Builder::spawn_scoped()`].
	#[allow(clippy::missing_errors_doc, single_use_lifetimes)]
	fn spawn_scoped_async<'scope, 'env, F1, F2, T>(
		self,
		scope: &'scope Scope<'scope, 'env>,
		#[allow(clippy::min_ident_chars)] f: F1,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F1: 'scope + FnOnce() -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send;
}

impl BuilderExt for Builder {
	fn spawn_async<F1, F2, T>(
		self,
		#[allow(clippy::min_ident_chars)] f: F1,
	) -> io::Result<JoinHandle<T>>
	where
		F1: 'static + FnOnce() -> F2 + Send,
		F2: 'static + Future<Output = T>,
		T: Send + 'static,
	{
		self.spawn_async_internal(f)
	}

	#[allow(single_use_lifetimes)]
	fn spawn_scoped_async<'scope, 'env, F1, F2, T>(
		self,
		scope: &'scope Scope<'scope, 'env>,
		#[allow(clippy::min_ident_chars)] f: F1,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F1: 'scope + FnOnce() -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
	{
		self.spawn_scoped_async_internal(scope, f)
	}
}

/// Web-specific extension to [`web_thread::Scope`](crate::Scope).
pub trait ScopeExt<'scope> {
	/// Async version of [`Scope::spawn()`].
	fn spawn_async<F1, F2, T>(
		&'scope self,
		#[allow(clippy::min_ident_chars)] f: F1,
	) -> ScopedJoinHandle<'scope, T>
	where
		F1: 'scope + FnOnce() -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send;
}

impl<'scope> ScopeExt<'scope> for Scope<'scope, '_> {
	fn spawn_async<F1, F2, T>(
		&'scope self,
		#[allow(clippy::min_ident_chars)] f: F1,
	) -> ScopedJoinHandle<'scope, T>
	where
		F1: 'scope + FnOnce() -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
	{
		self.spawn_async_internal(f)
	}
}

/// Async version of [`spawn()`](std::thread::spawn).
pub fn spawn_async<F1, F2, T>(#[allow(clippy::min_ident_chars)] f: F1) -> JoinHandle<T>
where
	F1: 'static + FnOnce() -> F2 + Send,
	F2: 'static + Future<Output = T>,
	T: Send + 'static,
{
	thread::spawn_async(f)
}
