//! Re-implementation of [`std::thread`].

#[cfg(target_feature = "atomics")]
mod atomics;
mod js;
#[cfg(not(target_feature = "atomics"))]
mod unsupported;
mod util;

use std::cell::OnceCell;
use std::fmt::{self, Debug, Formatter};
use std::io::{self, Error, ErrorKind};
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
pub use std::thread::*;
use std::time::Duration;

use r#impl::Parker;

#[cfg(target_feature = "atomics")]
use self::atomics as r#impl;
use self::js::CROSS_ORIGIN_ISOLATED;
#[cfg(not(target_feature = "atomics"))]
use self::unsupported as r#impl;
use self::util::{Global, GLOBAL};

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
	/// This function will always return an error if the atomics target
	/// feature is not enabled.
	#[allow(clippy::missing_errors_doc, clippy::type_repetition_in_bounds)]
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
				"operation not supported on this platform without the atomics target feature",
			))
		}
	}

	/// See [`std::thread::Builder::spawn_scoped()`].
	#[allow(clippy::missing_errors_doc, single_use_lifetimes)]
	pub fn spawn_scoped<'scope, 'env, F, T>(
		self,
		scope: &'scope Scope<'scope, 'env>,
		#[allow(clippy::min_ident_chars)] f: F,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F: FnOnce() -> T + Send + 'scope,
		T: Send + 'scope,
	{
		if has_spawn_support() {
			self.0.spawn_scoped(scope, f)
		} else {
			Err(Error::new(
				ErrorKind::Unsupported,
				"operation not supported on this platform without the atomics target feature",
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
pub struct JoinHandle<T>(pub(crate) r#impl::JoinHandle<T>);

impl<T> Debug for JoinHandle<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.debug_tuple("JoinHandle").field(&self.0).finish()
	}
}

impl<T> JoinHandle<T> {
	/// See [`std::thread::JoinHandle::is_finished()`].
	#[allow(clippy::must_use_candidate)]
	pub fn is_finished(&self) -> bool {
		self.0.is_finished()
	}

	/// See [`std::thread::JoinHandle::join()`].
	#[allow(clippy::missing_errors_doc)]
	pub fn join(self) -> Result<T> {
		self.0.join()
	}

	/// See [`std::thread::JoinHandle::thread()`].
	#[must_use]
	pub fn thread(&self) -> &Thread {
		self.0.thread()
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
	static THREAD: OnceCell<Thread> = OnceCell::new();
}

impl Thread {
	/// Create a new [`Thread`].
	fn new() -> Self {
		let name = GLOBAL
			.with(|global| match global.as_ref()? {
				Global::Dedicated(worker) => Some(worker.name()),
				Global::Shared(worker) => Some(worker.name()),
				Global::Window(_) | Global::Worklet => None,
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
pub struct ThreadId(u64);

impl ThreadId {
	/// Create a new [`ThreadId`].
	fn new() -> Self {
		/// Global counter for [`ThreadId`].
		static COUNTER: AtomicU64 = AtomicU64::new(1);

		Self(COUNTER.fetch_add(1, Ordering::Relaxed))
	}
}

/// See [`std::thread::available_parallelism()`].
///
/// # Notes
///
/// Browsers might use lower values, a common case is to prevent fingerprinting.
///
/// # Errors
///
/// This function will return an error if called from a worklet or any other
/// unsupported thread type.
#[allow(clippy::missing_panics_doc)]
pub fn available_parallelism() -> io::Result<NonZeroUsize> {
	let value = GLOBAL.with(|global| {
		let global = global.as_ref().ok_or_else(util::unsupported_global)?;

		match global {
			Global::Window(window) => Ok(window.navigator().hardware_concurrency()),
			Global::Dedicated(worker) => Ok(worker.navigator().hardware_concurrency()),
			Global::Shared(worker) => Ok(worker.navigator().hardware_concurrency()),
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
/// type.
pub fn park() {
	if has_wait_support() {
		// SAFETY: park_timeout is called on the parker owned by this thread.
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
/// unsupported thread type.
pub fn park_timeout(dur: Duration) {
	if has_wait_support() {
		// SAFETY: park_timeout is called on the parker owned by this thread.
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
/// unsupported thread type.
#[deprecated(note = "replaced by `web_thread::park_timeout`")]
pub fn park_timeout_ms(ms: u32) {
	park_timeout(Duration::from_millis(ms.into()));
}

/// See [`std::thread::scope()`].
#[track_caller]
pub fn scope<'env, F, T>(#[allow(clippy::min_ident_chars)] f: F) -> T
where
	F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T,
{
	r#impl::scope(f)
}

/// See [`std::thread::sleep()`].
///
/// # Panics
///
/// This call will panic unless called from a thread type that allows blocking,
/// e.g. a Web worker.
pub fn sleep(dur: Duration) {
	if has_wait_support() {
		r#impl::sleep(dur);
	} else {
		panic!("current thread type cannot be blocked")
	}
}

/// See [`std::thread::sleep_ms()`].
///
/// # Panics
///
/// This call will panic unless called from a thread type that allows blocking,
/// e.g. a Web worker.
#[deprecated(note = "replaced by `web_thread::sleep`")]
pub fn sleep_ms(ms: u32) {
	sleep(Duration::from_millis(ms.into()));
}

/// See [`std::thread::spawn()`].
///
/// # Errors
///
/// This function will always return an error if the atomics target
/// feature is not enabled.
#[allow(
	clippy::min_ident_chars,
	clippy::missing_panics_doc,
	clippy::type_repetition_in_bounds
)]
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
	F: FnOnce() -> T,
	F: Send + 'static,
	T: Send + 'static,
{
	Builder::new().spawn(f).expect("failed to spawn thread")
}

/// See [`std::thread::yield_now()`].
///
/// # Notes
///
/// This call is no-op.
pub fn yield_now() {
	thread::yield_now();
}

/// Implementation for
/// [`web::has_wait_support()`](crate::web::has_wait_support()).
pub(crate) fn has_wait_support() -> bool {
	GLOBAL.with(|global| {
		if global
			.as_ref()
			.filter(|global| global.is_worker())
			.is_some()
		{
			cfg!(target_feature = "atomics") || *CROSS_ORIGIN_ISOLATED.deref()
		} else {
			false
		}
	})
}

/// Implementation for [`crate::web::has_spawn_support()`].
pub(crate) fn has_spawn_support() -> bool {
	cfg!(target_feature = "atomics") && *CROSS_ORIGIN_ISOLATED.deref()
}
