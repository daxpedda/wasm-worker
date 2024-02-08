//! Re-implementation of [`std::thread`].

#[cfg(target_feature = "atomics")]
mod atomics;
#[cfg(feature = "audio-worklet")]
pub(crate) mod audio_worklet;
mod builder;
mod global;
mod js;
mod scope;
#[cfg(not(target_feature = "atomics"))]
mod unsupported;
mod yield_now;

use std::cell::OnceCell;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::io::{self, Error, ErrorKind};
use std::num::{NonZeroU64, NonZeroUsize};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll};
use std::thread;
use std::time::Duration;

use r#impl::Parker;

#[cfg(target_feature = "atomics")]
use self::atomics as r#impl;
pub use self::builder::Builder;
use self::global::{Global, GLOBAL};
pub use self::scope::{scope, Scope, ScopedJoinHandle};
pub(crate) use self::scope::{scope_async, ScopeFuture};
#[cfg(not(target_feature = "atomics"))]
use self::unsupported as r#impl;
pub use self::yield_now::yield_now;
pub(crate) use self::yield_now::YieldNowFuture;

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
	pub fn join(self) -> thread::Result<T> {
		self.0.join()
	}

	/// See [`std::thread::JoinHandle::thread()`].
	#[must_use]
	pub fn thread(&self) -> &Thread {
		self.0.thread()
	}

	/// Implementation for
	/// [`JoinHandleFuture::poll()`](crate::web::JoinHandleFuture).
	pub(crate) fn poll(&mut self, cx: &mut Context<'_>) -> Poll<thread::Result<T>> {
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
