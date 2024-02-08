//! Re-implementation of [`std::thread`].

#[cfg(target_feature = "atomics")]
mod atomics;
#[cfg(feature = "audio-worklet")]
pub(crate) mod audio_worklet;
mod global;
mod js;
#[cfg(not(target_feature = "atomics"))]
mod unsupported;

use std::cell::OnceCell;
use std::fmt::{self, Debug, Formatter};
use std::future::{Future, Ready};
use std::io::{self, Error, ErrorKind};
use std::marker::PhantomData;
use std::num::{NonZeroU64, NonZeroUsize};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::task::{ready, Context, Poll};
use std::thread::Result;
use std::time::Duration;
use std::{mem, thread};

use js_sys::{Object, Promise};
use pin_project::{pin_project, pinned_drop};
use r#impl::Parker;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{AbortController, MessageChannel, MessagePort};

#[cfg(target_feature = "atomics")]
use self::atomics as r#impl;
use self::global::{Global, GLOBAL};
use self::js::{SchedulerPostTaskOptions, TaskPriority, WindowExt};
#[cfg(not(target_feature = "atomics"))]
use self::unsupported as r#impl;
use crate::web::YieldTime;

/// See [`std::thread::Builder`].
#[derive(Debug)]
#[must_use = "must eventually spawn the thread"]
pub struct Builder(r#impl::Builder);

impl Builder {
	/// See [`std::thread::Builder::new()`].
	#[allow(clippy::new_without_default)]
	pub fn new() -> Self {
		Self(r#impl::Builder::new())
	}

	/// See [`std::thread::Builder::name()`].
	pub fn name(self, name: String) -> Self {
		Self(self.0.name(name))
	}

	/// See [`std::thread::Builder::spawn()`].
	///
	/// # Errors
	///
	/// If the main thread does not support spawning threads, see
	/// [`web::has_spawn_support()`](crate::web::has_spawn_support).
	#[allow(clippy::type_repetition_in_bounds)]
	pub fn spawn<F, T>(self, #[allow(clippy::min_ident_chars)] f: F) -> io::Result<JoinHandle<T>>
	where
		F: FnOnce() -> T,
		F: Send + 'static,
		T: Send + 'static,
	{
		if has_spawn_support() {
			self.0.spawn(f).map(JoinHandle)
		} else {
			Err(Error::new(
				ErrorKind::Unsupported,
				"operation not supported on this platform without the atomics target feature and \
				 cross-origin isolation",
			))
		}
	}

	/// Implementation for
	/// [`BuilderExt::spawn_async()`](crate::web::BuilderExt::spawn_async).
	pub(crate) fn spawn_async_internal<F1, F2, T>(self, task: F1) -> io::Result<JoinHandle<T>>
	where
		F1: 'static + FnOnce() -> F2 + Send,
		F2: 'static + Future<Output = T>,
		T: 'static + Send,
	{
		if has_spawn_support() {
			self.0.spawn_async_internal(task).map(JoinHandle)
		} else {
			Err(Error::new(
				ErrorKind::Unsupported,
				"operation not supported on this platform without the atomics target feature and \
				 cross-origin isolation",
			))
		}
	}

	/// See [`std::thread::Builder::spawn_scoped()`].
	///
	/// # Errors
	///
	/// If the main thread does not support spawning threads, see
	/// [`web::has_spawn_support()`](crate::web::has_spawn_support).
	pub fn spawn_scoped<'scope, #[allow(single_use_lifetimes)] 'env, F, T>(
		self,
		scope: &'scope Scope<'scope, 'env>,
		#[allow(clippy::min_ident_chars)] f: F,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F: FnOnce() -> T + Send + 'scope,
		T: Send + 'scope,
	{
		if has_spawn_support() {
			self.0.spawn_scoped(&scope.this, f)
		} else {
			Err(Error::new(
				ErrorKind::Unsupported,
				"operation not supported on this platform without the atomics target feature and \
				 cross-origin isolation",
			))
		}
	}

	/// Implementation for
	/// [`BuilderExt::spawn_scoped_async()`](crate::web::BuilderExt::spawn_scoped_async).
	pub(crate) fn spawn_scoped_async_internal<'scope, F1, F2, T>(
		self,
		scope: &'scope Scope<'scope, '_>,
		task: F1,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F1: 'scope + FnOnce() -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
	{
		if has_spawn_support() {
			self.0.spawn_scoped_async_internal(&scope.this, task)
		} else {
			Err(Error::new(
				ErrorKind::Unsupported,
				"operation not supported on this platform without the atomics target feature and \
				 cross-origin isolation",
			))
		}
	}

	/// See [`std::thread::Builder::stack_size()`].
	///
	/// # Notes
	///
	/// This call is no-op. The default stack size is 1MB. To modify the stack
	/// size allocated per thread use the `WASM_BINDGEN_THREADS_STACK_SIZE`
	/// environment variable when executing `wasm-bindgen-cli`.
	#[allow(clippy::missing_const_for_fn)]
	pub fn stack_size(self, #[allow(unused)] size: usize) -> Self {
		self
	}
}

/// See [`std::thread::JoinHandle`].
pub struct JoinHandle<T>(r#impl::JoinHandle<T>);

impl<T> Debug for JoinHandle<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.debug_tuple("JoinHandle").field(&self.0).finish()
	}
}

impl<T> JoinHandle<T> {
	/// See [`std::thread::JoinHandle::is_finished()`].
	///
	/// # Notes
	///
	/// When this returns [`true`] it guarantees [`JoinHandle::join()`] not to
	/// block.
	#[allow(clippy::must_use_candidate)]
	pub fn is_finished(&self) -> bool {
		self.0.is_finished()
	}

	/// See [`std::thread::JoinHandle::join()`].
	///
	/// # Notes
	///
	/// When compiling with [`panic = "abort"`], which is the only option
	/// without enabling the Wasm exception-handling proposal, this can never
	/// return [`Err`].
	///
	/// # Panics
	///
	/// - If the calling thread doesn't support blocking, see
	///   [`web::has_block_support()`](crate::web::has_block_support). Though it
	///   is guaranteed to not block if [`JoinHandle::is_finished()`] returns
	///   [`true`]. Alternatively consider using
	///   [`web::JoinHandleExt::join_async()`].
	/// - If called on the thread to join.
	/// - If it was already polled to completion through
	///   [`web::JoinHandleExt::join_async()`].
	///
	/// [`panic = "abort"`]: https://doc.rust-lang.org/1.75.0/cargo/reference/profiles.html#panic
	/// [`web::JoinHandleExt::join_async()`]: crate::web::JoinHandleExt::join_async
	#[allow(clippy::missing_errors_doc)]
	pub fn join(self) -> Result<T> {
		self.0.join()
	}

	/// See [`std::thread::JoinHandle::thread()`].
	#[must_use]
	pub fn thread(&self) -> &Thread {
		self.0.thread()
	}

	/// Implementation for
	/// [`JoinHandleFuture::poll()`](crate::web::JoinHandleFuture).
	pub(crate) fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<T>> {
		Pin::new(&mut self.0).poll(cx)
	}
}

/// See [`std::thread::Thread`].
#[derive(Clone, Debug)]
pub struct Thread(Arc<ThreadInner>);

/// Inner shared wrapper for [`Thread`].
#[derive(Debug)]
struct ThreadInner {
	/// [`ThreadId`].
	id: ThreadId,
	/// Name of the thread.
	name: Option<String>,
	/// Parker implementation.
	parker: Parker,
}

thread_local! {
	/// Holds this threads [`Thread`].
	static THREAD: OnceCell<Thread> = const { OnceCell::new() };
}

impl Thread {
	/// Create a new [`Thread`].
	fn new() -> Self {
		let name = GLOBAL
			.with(|global| match global.as_ref()? {
				Global::Dedicated(worker) => Some(worker.name()),
				Global::Shared(worker) => Some(worker.name()),
				Global::Window(_) | Global::Service(_) | Global::Worklet | Global::Worker(_) => {
					None
				}
			})
			.filter(|name| !name.is_empty());

		Self::new_with_name(name)
	}

	/// Create a new [`Thread`].
	fn new_with_name(name: Option<String>) -> Self {
		Self(Arc::new(ThreadInner {
			id: ThreadId::new(),
			name,
			parker: Parker::new(),
		}))
	}

	/// See [`std::thread::Thread::id()`].
	#[must_use]
	pub fn id(&self) -> ThreadId {
		self.0.id
	}

	/// See [`std::thread::Thread::name()`].
	#[must_use]
	pub fn name(&self) -> Option<&str> {
		self.0.name.as_deref()
	}

	/// See [`std::thread::Thread::unpark()`].
	#[inline]
	pub fn unpark(&self) {
		self.0.parker.unpark();
	}
}

/// See [`std::thread::ThreadId`].
#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct ThreadId(NonZeroU64);

impl ThreadId {
	/// Create a new [`ThreadId`].
	fn new() -> Self {
		// See <https://github.com/rust-lang/rust/blob/1.75.0/library/std/src/thread/mod.rs#L1177-L1218>.

		/// Separate failed [`ThreadId`] to apply `#[cold]` to it.
		#[cold]
		fn exhausted() -> ! {
			panic!("failed to generate unique thread ID: bitspace exhausted")
		}

		/// Global counter for [`ThreadId`].
		static COUNTER: AtomicU64 = AtomicU64::new(0);

		let mut last = COUNTER.load(Ordering::Relaxed);
		loop {
			let Some(id) = last.checked_add(1) else {
				exhausted();
			};

			match COUNTER.compare_exchange_weak(last, id, Ordering::Relaxed, Ordering::Relaxed) {
				Ok(_) => return Self(NonZeroU64::new(id).expect("unexpected `0` `ThreadId`")),
				Err(id) => last = id,
			}
		}
	}
}

/// See [`std::thread::Scope`].
#[derive(Debug)]
pub struct Scope<'scope, 'env: 'scope> {
	/// Implementation of [`Scope`].
	this: r#impl::Scope,
	/// Invariance over the lifetime `'scope`.
	#[allow(clippy::struct_field_names)]
	_scope: PhantomData<&'scope mut &'scope ()>,
	/// Invariance over the lifetime `'env`.
	_env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, #[allow(single_use_lifetimes)] 'env> Scope<'scope, 'env> {
	/// See [`std::thread::Scope::spawn()`].
	///
	/// # Panics
	///
	/// See [`spawn()`].
	pub fn spawn<F, T>(
		&'scope self,
		#[allow(clippy::min_ident_chars)] f: F,
	) -> ScopedJoinHandle<'scope, T>
	where
		F: FnOnce() -> T + Send + 'scope,
		T: Send + 'scope,
	{
		Builder::new()
			.spawn_scoped(self, f)
			.expect("failed to spawn thread")
	}

	/// Implementation for
	/// [`ScopeExt::spawn_async()`](crate::web::ScopeExt::spawn_async).
	pub(crate) fn spawn_async_internal<F1, F2, T>(
		&'scope self,
		task: F1,
	) -> ScopedJoinHandle<'scope, T>
	where
		F1: 'scope + FnOnce() -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
	{
		Builder::new()
			.spawn_scoped_async_internal(self, task)
			.expect("failed to spawn thread")
	}
}

/// See [`std::thread::ScopedJoinHandle`].
pub struct ScopedJoinHandle<'scope, T> {
	/// The underlying [`JoinHandle`].
	handle: r#impl::JoinHandle<T>,
	/// Hold the `'scope` lifetime.
	_scope: PhantomData<&'scope ()>,
}

impl<T> Debug for ScopedJoinHandle<'_, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("ScopedJoinHandle")
			.field("handle", &self.handle)
			.field("_scope", &self._scope)
			.finish()
	}
}

impl<#[allow(single_use_lifetimes)] 'scope, T> ScopedJoinHandle<'scope, T> {
	/// See [`std::thread::ScopedJoinHandle::thread()`].
	#[must_use]
	pub fn thread(&self) -> &Thread {
		self.handle.thread()
	}

	/// See [`std::thread::ScopedJoinHandle::join()`].
	///
	/// # Notes
	///
	/// When compiling with [`panic = "abort"`], which is the only option
	/// without enabling the Wasm exception-handling proposal, this can never
	/// return [`Err`].
	///
	/// # Panics
	///
	/// - If the calling thread doesn't support blocking, see
	///   [`web::has_block_support()`](crate::web::has_block_support). Though it
	///   is guaranteed to not block if [`ScopedJoinHandle::is_finished()`]
	///   returns [`true`]. Alternatively consider using
	///   [`web::ScopedJoinHandleExt::join_async()`].
	/// - If called on the thread to join.
	/// - If it was already polled to completion through
	///   [`web::ScopedJoinHandleExt::join_async()`].
	///
	/// [`panic = "abort"`]: https://doc.rust-lang.org/1.75.0/cargo/reference/profiles.html#panic
	/// [`web::ScopedJoinHandleExt::join_async()`]: crate::web::ScopedJoinHandleExt::join_async
	#[allow(clippy::missing_errors_doc)]
	pub fn join(self) -> Result<T> {
		self.handle.join()
	}

	/// See [`std::thread::ScopedJoinHandle::is_finished()`].
	///
	/// # Notes
	///
	/// When this returns [`true`] it guarantees [`ScopedJoinHandle::join()`]
	/// not to block.
	#[allow(clippy::must_use_candidate)]
	pub fn is_finished(&self) -> bool {
		self.handle.is_finished()
	}

	/// Implementation for
	/// [`ScopedJoinHandleFuture::poll()`](crate::web::ScopedJoinHandleFuture).
	pub(crate) fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<T>> {
		Pin::new(&mut self.handle).poll(cx)
	}
}

/// See [`std::thread::available_parallelism()`].
///
/// # Notes
///
/// Browsers might return lower values, a common case is to prevent
/// fingerprinting.
///
/// # Errors
///
/// This function will return an error if called from a worklet or any other
/// unsupported thread type.
#[allow(clippy::missing_panics_doc)]
pub fn available_parallelism() -> io::Result<NonZeroUsize> {
	let value = GLOBAL.with(|global| {
		let global = global.as_ref().ok_or_else(|| {
			Error::new(
				ErrorKind::Unsupported,
				"encountered unsupported thread type",
			)
		})?;

		match global {
			Global::Window(window) => Ok(window.navigator().hardware_concurrency()),
			Global::Dedicated(worker) => Ok(worker.navigator().hardware_concurrency()),
			Global::Shared(worker) => Ok(worker.navigator().hardware_concurrency()),
			Global::Service(worker) | Global::Worker(worker) => {
				Ok(worker.navigator().hardware_concurrency())
			}
			Global::Worklet => Err(Error::new(
				ErrorKind::Unsupported,
				"operation not supported in worklets",
			)),
		}
	})?;

	#[allow(
		clippy::as_conversions,
		clippy::cast_possible_truncation,
		clippy::cast_sign_loss
	)]
	let value = value as usize;
	let value = NonZeroUsize::new(value)
		.expect("`Navigator.hardwareConcurrency` returned an unexpected value of `0`");

	Ok(value)
}

/// See [`std::thread::current()`].
#[must_use]
pub fn current() -> Thread {
	THREAD.with(|cell| cell.get_or_init(Thread::new).clone())
}

/// See [`std::thread::park()`].
///
/// # Notes
///
/// Unlike [`std::thread::park()`], when using the atomics target feature, this
/// will not panic on the main thread, worklet or any other unsupported thread
/// type. However, on supported thread types, this will function correctly even
/// without the atomics target feature.
///
/// Keep in mind that this call will do nothing unless the calling thread
/// supports blocking, see
/// [`web::has_block_support()`](crate::web::has_block_support).
pub fn park() {
	if has_block_support() {
		// SAFETY: `park` is called on the parker owned by this thread.
		unsafe {
			current().0.parker.park();
		}
	}
}

/// See [`std::thread::park_timeout()`].
///
/// # Notes
///
/// Unlike [`std::thread::park_timeout()`], when using the atomics target
/// feature, this will not panic on the main thread, worklet or any other
/// unsupported thread type. However, on supported thread types, this will
/// function correctly even without the atomics target feature.
///
/// Keep in mind that this call will do nothing unless the calling thread
/// supports blocking, see
/// [`web::has_block_support()`](crate::web::has_block_support).
pub fn park_timeout(dur: Duration) {
	if has_block_support() {
		// SAFETY: `park_timeout` is called on the parker owned by this thread.
		unsafe {
			current().0.parker.park_timeout(dur);
		}
	}
}

/// See [`std::thread::park_timeout_ms()`].
///
/// # Notes
///
/// Unlike [`std::thread::park_timeout_ms()`], when using the atomics target
/// feature, this will not panic on the main thread, worklet or any other
/// unsupported thread type. However, on supported thread types, this will
/// function correctly even without the atomics target feature.
///
/// Keep in mind that this call will do nothing unless the calling thread
/// supports blocking, see
/// [`web::has_block_support()`](crate::web::has_block_support).
#[deprecated(note = "replaced by `web_thread::park_timeout`")]
pub fn park_timeout_ms(ms: u32) {
	park_timeout(Duration::from_millis(ms.into()));
}

/// See [`std::thread::scope()`].
///
/// # Notes
///
/// Keep in mind that this will enter a spinloop until all threads are joined if
/// blocking is not supported on this thread, see
/// [`web::has_block_support()`](crate::web::has_block_support).
///
/// Alternatively consider using
/// [`web::scope_async()`](crate::web::scope_async).
#[track_caller]
pub fn scope<'env, F, T>(#[allow(clippy::min_ident_chars)] f: F) -> T
where
	F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T,
{
	let scope = Scope {
		this: r#impl::Scope::new(),
		_scope: PhantomData,
		_env: PhantomData,
	};
	let result = f(&scope);

	scope.this.finish();

	result
}

/// Implementation for [`crate::web::scope_async()`].
pub(crate) fn scope_async<'scope, 'env: 'scope, F1, F2, T>(
	task: F1,
) -> ScopeFuture<'scope, 'env, F2, T>
where
	F1: FnOnce(&'scope Scope<'scope, 'env>) -> F2,
	F2: Future<Output = T>,
{
	let scope = Box::pin(Scope {
		this: r#impl::Scope::new(),
		_scope: PhantomData,
		_env: PhantomData,
	});
	// SAFETY: We have to make sure that `task` is dropped and all threads have
	// finished before `scope` is dropped.
	let task = task(unsafe { mem::transmute(scope.as_ref()) });

	ScopeFuture(ScopeFutureInner::Task { task, scope })
}

/// Waits for the associated scope to finish.
#[pin_project(PinnedDrop)]
pub(crate) struct ScopeFuture<'scope, 'env, F, T>(#[pin] ScopeFutureInner<'scope, 'env, F, T>);

/// State for [`ScopeFuture`].
#[pin_project(project = ScopeFutureProj, project_replace = ScopeFutureReplace)]
enum ScopeFutureInner<'scope, 'env, F, T> {
	/// Executing the task given to [`scope_async()`].
	Task {
		/// [`Future`] given by the user.
		#[pin]
		task: F,
		/// Corresponding [`Scope`].
		scope: Pin<Box<Scope<'scope, 'env>>>,
	},
	/// Wait for all threads to finish.
	Wait {
		/// Result of the [`Future`] given by the user.
		result: T,
		/// Corresponding [`Scope`].
		scope: Pin<Box<Scope<'scope, 'env>>>,
	},
	/// [`Future`] was polled to conclusion.
	None,
}

impl<F, T> Debug for ScopeFuture<'_, '_, F, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.debug_tuple("ScopeFuture").field(&self.0).finish()
	}
}

impl<F, T> Debug for ScopeFutureInner<'_, '_, F, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Task { scope, .. } => formatter
				.debug_struct("Task")
				.field("scope", &scope)
				.finish_non_exhaustive(),
			Self::Wait { scope, .. } => formatter
				.debug_struct("Wait")
				.field("scope", &scope)
				.finish_non_exhaustive(),
			Self::None => formatter.write_str("None"),
		}
	}
}

#[pinned_drop]
impl<F, T> PinnedDrop for ScopeFuture<'_, '_, F, T> {
	fn drop(self: Pin<&mut Self>) {
		let this = self.project();

		// SAFETY: We have to make sure that `task` is dropped and all threads have
		// finished before `scope` is dropped.
		if let ScopeFutureReplace::Task { scope, .. } | ScopeFutureReplace::Wait { scope, .. } =
			this.0.project_replace(ScopeFutureInner::None)
		{
			scope.this.finish();
		}
	}
}

impl<F, T> Future for ScopeFuture<'_, '_, F, T>
where
	F: Future<Output = T>,
{
	type Output = T;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let mut this = self.project();

		loop {
			match this.0.as_mut().project() {
				ScopeFutureProj::Task { task, .. } => {
					let result = ready!(task.poll(cx));
					let ScopeFutureReplace::Task { scope, .. } =
						this.0.as_mut().project_replace(ScopeFutureInner::None)
					else {
						unreachable!("found wrong state")
					};
					this.0
						.as_mut()
						.project_replace(ScopeFutureInner::Wait { result, scope });
				}
				ScopeFutureProj::Wait { scope, .. } => {
					ready!(scope.this.finish_async(cx));
					// SAFETY: We have to make sure that `task` is dropped and all threads have
					// finished before `scope` is dropped.
					let ScopeFutureReplace::Wait { result, .. } =
						this.0.project_replace(ScopeFutureInner::None)
					else {
						unreachable!("found wrong state")
					};
					return Poll::Ready(result);
				}
				ScopeFutureProj::None => panic!("`ScopeFuture` polled after completion"),
			}
		}
	}
}

impl<'scope, 'env, F, T> ScopeFuture<'scope, 'env, F, T>
where
	F: Future<Output = T>,
{
	/// Implementation for
	/// [`ScopeFuture::into_wait()`](crate::web::ScopeFuture::into_wait).
	pub(crate) fn poll_into_wait(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<ScopeFuture<'scope, 'env, Ready<T>, T>> {
		let mut this = self.project();

		match this.0.as_mut().project() {
			ScopeFutureProj::Task { task, .. } => {
				let result = ready!(task.poll(cx));
				let ScopeFutureReplace::Task { scope, .. } =
					this.0.project_replace(ScopeFutureInner::None)
				else {
					unreachable!("found wrong state")
				};
				Poll::Ready(ScopeFuture(ScopeFutureInner::Wait { result, scope }))
			}
			ScopeFutureProj::Wait { .. } => {
				let ScopeFutureReplace::Wait { result, scope } =
					this.0.project_replace(ScopeFutureInner::None)
				else {
					unreachable!("found wrong state")
				};
				return Poll::Ready(ScopeFuture(ScopeFutureInner::Wait { result, scope }));
			}
			ScopeFutureProj::None => panic!("`ScopeFuture` polled after completion"),
		}
	}

	/// Implementation for
	/// [`ScopeJoinFuture::is_finished()`](crate::web::ScopeJoinFuture::is_finished).
	pub(crate) fn is_finished(&self) -> bool {
		match &self.0 {
			ScopeFutureInner::Task { .. } => false,
			ScopeFutureInner::Wait { scope, .. } => scope.this.thread_count() == 0,
			ScopeFutureInner::None => true,
		}
	}

	/// Implementation for
	/// [`ScopeJoinFuture::join_all()`](crate::web::ScopeJoinFuture::join_all).
	pub(crate) fn join_all(mut self) -> T {
		match mem::replace(&mut self.0, ScopeFutureInner::None) {
			ScopeFutureInner::Wait { result, scope } => {
				assert!(has_block_support(), "current thread type cannot be blocked");

				scope.this.finish();
				result
			}
			ScopeFutureInner::None => {
				panic!("called after `ScopeJoinFuture` was polled to completion")
			}
			ScopeFutureInner::Task { .. } => {
				unreachable!("should only be called from `ScopeJoinFuture`")
			}
		}
	}
}

/// See [`std::thread::sleep()`].
///
/// # Panics
///
/// This call will panic if the calling thread doesn't support blocking, see
/// [`web::has_block_support()`](crate::web::has_block_support).
pub fn sleep(dur: Duration) {
	if has_block_support() {
		r#impl::sleep(dur);
	} else {
		panic!("current thread type cannot be blocked")
	}
}

/// See [`std::thread::sleep_ms()`].
///
/// # Panics
///
/// This call will panic if the calling thread doesn't support blocking, see
/// [`web::has_block_support()`](crate::web::has_block_support).
#[deprecated(note = "replaced by `web_thread::sleep`")]
pub fn sleep_ms(ms: u32) {
	sleep(Duration::from_millis(ms.into()));
}

/// See [`std::thread::spawn()`].
///
/// # Panics
///
/// If the main thread does not support spawning threads, see
/// [`web::has_spawn_support()`](crate::web::has_spawn_support).
#[allow(clippy::min_ident_chars, clippy::type_repetition_in_bounds)]
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
	F: FnOnce() -> T,
	F: Send + 'static,
	T: Send + 'static,
{
	Builder::new().spawn(f).expect("failed to spawn thread")
}

/// Implementation for [`crate::web::spawn_async()`].
pub(crate) fn spawn_async<F1, F2, T>(task: F1) -> JoinHandle<T>
where
	F1: 'static + FnOnce() -> F2 + Send,
	F2: 'static + Future<Output = T>,
	T: 'static + Send,
{
	Builder::new()
		.spawn_async_internal(task)
		.expect("failed to spawn thread")
}

/// See [`std::thread::yield_now()`].
///
/// # Notes
///
/// This call is no-op. Alternatively consider using
/// [`web::yield_now_async()`](crate::web::yield_now_async).
pub fn yield_now() {
	thread::yield_now();
}

/// Implementation for [`crate::web::has_block_support()`].
pub(crate) fn has_block_support() -> bool {
	thread_local! {
		static HAS_BLOCK_SUPPORT: bool = GLOBAL
			.with(|global| {
				global.as_ref().and_then(|global| match global {
					Global::Window(_) | Global::Worklet | Global::Service(_) => Some(false),
					Global::Dedicated(_) => Some(true),
					// Firefox doesn't support blocking in shared workers, so for cross-browser
					// support we have to test manually.
					// See <https://bugzilla.mozilla.org/show_bug.cgi?id=1359745>.
					Global::Shared(_) => {
						/// Cache if blocking on shared workers is supported.
						static HAS_SHARED_WORKER_BLOCK_SUPPORT: OnceLock<bool> = OnceLock::new();

						Some(*HAS_SHARED_WORKER_BLOCK_SUPPORT.get_or_init(r#impl::test_block_support))
					}
					Global::Worker(_) => None,
				})
			})
			// For unknown worker types we test manually.
			.unwrap_or_else(r#impl::test_block_support);
	}

	HAS_BLOCK_SUPPORT.with(bool::clone)
}

/// Implementation for [`crate::web::has_spawn_support()`].
pub(crate) fn has_spawn_support() -> bool {
	r#impl::has_spawn_support()
}

/// Implementation for [`crate::web::yield_now_async()`].
pub(crate) fn yield_now_async(priority: YieldTime) -> YieldNowFuture {
	thread_local! {
		static HAS_SCHEDULER: bool = Global::with_window_or_worker(|global| !global.has_scheduler().is_undefined()).unwrap_or(false);
		static HAS_REQUEST_IDLE_CALLBACK: bool = GLOBAL.with(|global| {
			if let Some(Global::Window(window)) = global.as_ref() {
				let window: &WindowExt = window.unchecked_ref();
				!window.has_request_idle_callback().is_undefined()
			} else {
				false
			}
		});
		static EMPTY_CLOSURE: Closure<dyn FnMut(JsValue)> = Closure::new(|_| ());
	}

	match priority {
		YieldTime::UserBlocking | YieldTime::UserVisible | YieldTime::Background
			if HAS_SCHEDULER.with(bool::clone) =>
		{
			Global::with_window_or_worker(|global| {
				let options: SchedulerPostTaskOptions = Object::new().unchecked_into();
				let controller =
					AbortController::new().expect("`new AbortController` is not expected to fail");
				options.set_signal(&controller.signal());

				match priority {
					YieldTime::UserBlocking => options.set_priority(TaskPriority::UserBlocking),
					YieldTime::UserVisible => (),
					YieldTime::Background => options.set_priority(TaskPriority::Background),
					YieldTime::Idle => unreachable!("found invalid `YieldTime`"),
				}

				let future = JsFuture::from(EMPTY_CLOSURE.with(|closure| {
					global
						.scheduler()
						.post_task_with_options(closure.as_ref().unchecked_ref(), &options)
						.catch(closure)
				}));

				YieldNowFuture(Some(State::Scheduler { future, controller }))
			})
			.expect("found invalid global context despite previous check")
		}
		YieldTime::Idle if HAS_REQUEST_IDLE_CALLBACK.with(bool::clone) => GLOBAL.with(|global| {
			let Some(Global::Window(window)) = global.as_ref() else {
				unreachable!("expected `Window`")
			};
			let mut handle = None;
			let future = JsFuture::from(Promise::new(&mut |resolve, _| {
				handle = Some(
					window
						.request_idle_callback(&resolve)
						.expect("`setTimeout` is not expected to fail"),
				);
			}));
			let handle = handle.expect("Callback passed into `Promise` not executed immediately");

			YieldNowFuture(Some(State::Idle { future, handle }))
		}),
		// `MessageChannel` can't be instantiated in a worklet.
		// TODO: Send a `MessageChannel` to a Worklet to make this possible.
		_ => Global::with_window_or_worker(|_| {
			let channel =
				MessageChannel::new().expect("`new MessageChannel` is not expected to fail");
			let port1 = channel.port1();
			let future = JsFuture::from(Promise::new(&mut |resolve, _| {
				port1.set_onmessage(Some(&resolve));
			}));
			channel
				.port2()
				.post_message(&JsValue::UNDEFINED)
				.expect("failed to send empty message");

			YieldNowFuture(Some(State::Channel {
				future,
				port: port1,
			}))
		})
		.unwrap_or(YieldNowFuture(Some(State::None))),
	}
}

/// Waits for yielding to the event loop to happen.
#[derive(Debug)]
pub(crate) struct YieldNowFuture(Option<State>);

/// State of [`YieldNowFuture`].
#[derive(Debug)]
enum State {
	/// Used [`Scheduler.postTask()`](https://developer.mozilla.org/en-US/docs/Web/API/Scheduler/postTask).
	Scheduler {
		/// [`Future`].
		future: JsFuture,
		/// Abort when dropped.
		controller: AbortController,
	},
	/// Used [`Window.requestIdleCallback()`](https://developer.mozilla.org/en-US/docs/Web/API/Window/requestIdleCallback).
	Idle {
		/// [`Future`].
		future: JsFuture,
		/// Abort when dropped.
		handle: u32,
	},
	/// Used [`MessagePort.postMessage()`](https://developer.mozilla.org/en-US/docs/Web/API/MessagePort/postMessage).
	Channel {
		/// [`Future`].
		future: JsFuture,
		/// Abort when dropped.
		port: MessagePort,
	},
	/// Yielding to the event loop not supported.
	None,
}

impl Drop for YieldNowFuture {
	fn drop(&mut self) {
		if let Some(state) = self.0.take() {
			match state {
				State::Scheduler { controller, .. } => controller.abort(),
				State::Idle { handle, .. } => GLOBAL.with(|global| {
					let Some(Global::Window(window)) = global.as_ref() else {
						unreachable!("expected `Window`")
					};
					window.cancel_idle_callback(handle);
				}),
				State::Channel { port, .. } => port.close(),
				State::None => (),
			}
		}
	}
}

impl Future for YieldNowFuture {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		match self
			.0
			.as_mut()
			.expect("`YieldNowFuture` polled after completion")
		{
			State::Scheduler { future, .. }
			| State::Idle { future, .. }
			| State::Channel { future, .. } => {
				ready!(Pin::new(future).poll(cx)).expect("unexpected failure in empty `Promise`");
			}
			State::None => (),
		}

		self.0.take().expect("found empty `State`");
		Poll::Ready(())
	}
}
