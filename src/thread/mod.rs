//! Re-implementation of [`std::thread`].

#[cfg(target_feature = "atomics")]
mod atomics;
mod js;

use std::cell::OnceCell;
use std::fmt::{self, Debug, Formatter};
use std::io::{self, Error, ErrorKind};
#[cfg(not(target_feature = "atomics"))]
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
#[cfg(target_feature = "atomics")]
use std::sync::PoisonError;
use std::thread;
pub use std::thread::*;
use std::time::Duration;

#[cfg(not(target_feature = "atomics"))]
use js_sys::{Atomics, Int32Array, SharedArrayBuffer};

use self::js::{Global, GLOBAL};

/// See [`std::thread::Builder`].
#[derive(Debug)]
#[must_use = "must eventually spawn the thread"]
pub struct Builder {
	/// Name of the thread.
	name: Option<String>,
}

impl Builder {
	/// See [`std::thread::Builder::new()`].
	#[allow(clippy::missing_const_for_fn, clippy::new_without_default)]
	pub fn new() -> Self {
		Self { name: None }
	}

	/// See [`std::thread::Builder::name()`].
	pub fn name(mut self, name: String) -> Self {
		self.name = Some(name);
		self
	}

	/// See [`std::thread::Builder::spawn()`].
	///
	/// # Errors
	///
	/// This function will always return an error if the atomics target
	/// feature is not enabled.
	#[allow(clippy::missing_errors_doc, clippy::type_repetition_in_bounds)]
	#[cfg_attr(not(target_feature = "atomics"), allow(clippy::unused_self))]
	pub fn spawn<F, T>(
		self,
		#[allow(clippy::min_ident_chars)]
		#[cfg_attr(not(target_feature = "atomics"), allow(unused))]
		f: F,
	) -> io::Result<JoinHandle<T>>
	where
		F: FnOnce() -> T,
		F: Send + 'static,
		T: Send + 'static,
	{
		#[cfg(target_feature = "atomics")]
		{
			atomics::spawn(f, self.name)
		}
		#[cfg(not(target_feature = "atomics"))]
		Err(Error::new(
			ErrorKind::Unsupported,
			"operation not supported on this platform without the atomics target feature",
		))
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
		#[cfg(target_feature = "atomics")]
		{
			todo!()
		}
		#[cfg(not(target_feature = "atomics"))]
		Err(Error::new(
			ErrorKind::Unsupported,
			"operation not supported on this platform without the atomics target feature",
		))
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
pub struct JoinHandle<T> {
	/// Shared state between [`JoinHandle`] and thread.
	#[cfg(target_feature = "atomics")]
	pub(crate) shared: Arc<atomics::Shared<T>>,
	/// Corresponding [`Thread`].
	#[cfg(target_feature = "atomics")]
	pub(crate) thread: Thread,
	#[cfg(not(target_feature = "atomics"))]
	_value: PhantomData<T>,
}

impl<T> Debug for JoinHandle<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		let mut debug = formatter.debug_struct("JoinHandle");

		#[cfg(target_feature = "atomics")]
		{
			debug
				.field("shared", &self.shared)
				.field("thread", &self.thread);
		}
		#[cfg(not(target_feature = "atomics"))]
		{
			let _ = debug.finish_non_exhaustive();
		}

		debug.finish()
	}
}

impl<T> JoinHandle<T> {
	/// See [`std::thread::JoinHandle::is_finished()`].
	#[allow(clippy::must_use_candidate)]
	#[cfg_attr(not(target_feature = "atomics"), allow(clippy::unused_self))]
	pub fn is_finished(&self) -> bool {
		#[cfg(target_feature = "atomics")]
		{
			Arc::strong_count(&self.shared) == 1
		}
		#[cfg(not(target_feature = "atomics"))]
		unreachable!("found instanced `JoinHandle` without threading support")
	}

	/// See [`std::thread::JoinHandle::join()`].
	#[allow(
		clippy::missing_errors_doc,
		clippy::missing_panics_doc,
		clippy::unnecessary_wraps
	)]
	#[cfg_attr(not(target_feature = "atomics"), allow(clippy::unused_self))]
	pub fn join(self) -> Result<T> {
		#[cfg(target_feature = "atomics")]
		{
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
		#[cfg(not(target_feature = "atomics"))]
		unreachable!("found instanced `JoinHandle` without threading support")
	}

	/// See [`std::thread::JoinHandle::thread()`].
	#[must_use]
	#[cfg_attr(target_feature = "atomics", allow(clippy::missing_const_for_fn))]
	#[cfg_attr(not(target_feature = "atomics"), allow(clippy::unused_self))]
	pub fn thread(&self) -> &Thread {
		#[cfg(target_feature = "atomics")]
		{
			&self.thread
		}
		#[cfg(not(target_feature = "atomics"))]
		unreachable!("found instanced `JoinHandle` without threading support")
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
	#[cfg(target_feature = "atomics")]
	parker: atomics::Parker,
}

thread_local! {
	/// Holds this threads [`Thread`].
	static THREAD: OnceCell<Thread> = OnceCell::new();
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
			#[cfg(target_feature = "atomics")]
			parker: atomics::Parker::new(),
		}))
	}

	/// Gets the current [`Thread`] and instantiates it if not set.
	fn current() -> Self {
		THREAD.with(|cell| cell.get_or_init(Self::new).clone())
	}

	/// Registers the given `thread`.
	#[cfg(target_feature = "atomics")]
	fn register(thread: Self) {
		THREAD.with(|cell| cell.set(thread).expect("`Thread` already registered"));
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
	#[cfg_attr(
		not(target_feature = "atomics"),
		allow(clippy::missing_const_for_fn, clippy::unused_self)
	)]
	pub fn unpark(&self) {
		#[cfg(target_feature = "atomics")]
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
		let global = global.as_ref().ok_or_else(js::unsupported_global)?;

		match global {
			Global::Window(window) => Ok(window.navigator().hardware_concurrency()),
			Global::Worker(worker) => Ok(worker.navigator().hardware_concurrency()),
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
	Thread::current()
}

/// See [`std::thread::park()`].
///
/// # Notes
///
/// Unlike [`std::thread::park()`] this will not panic on the main thread,
/// worklet or any other unsupported thread type when using `target_feature =
/// "atomics"`.
#[cfg_attr(not(target_feature = "atomics"), allow(clippy::missing_const_for_fn))]
pub fn park() {
	#[cfg(target_feature = "atomics")]
	{
		GLOBAL.with(|global| {
			if let Some(Global::Worker(_)) = global {
				// SAFETY: park_timeout is called on the parker owned by this thread.
				unsafe {
					current().0.parker.park();
				}
			}
		});
	}
}

/// See [`std::thread::park_timeout()`].
///
/// # Notes
///
/// Unlike [`std::thread::park_timeout()`] this will not panic on the main
/// thread, worklet or any other unsupported thread type when using
/// `target_feature = "atomics"`.
#[cfg_attr(not(target_feature = "atomics"), allow(clippy::missing_const_for_fn))]
pub fn park_timeout(
	#[cfg_attr(not(target_feature = "atomics"), allow(unused_variables))] dur: Duration,
) {
	#[cfg(target_feature = "atomics")]
	{
		GLOBAL.with(|global| {
			if let Some(Global::Worker(_)) = global {
				// SAFETY: park_timeout is called on the parker owned by this thread.
				unsafe {
					current().0.parker.park_timeout(dur);
				}
			}
		});
	}
}

/// See [`std::thread::park_timeout_ms()`].
///
/// # Notes
///
/// Unlike [`std::thread::park_timeout_ms()`] this will not panic on the main
/// thread, worklet or any other unsupported thread type when using
/// `target_feature = "atomics"`.
#[deprecated(note = "replaced by `web_thread::park_timeout`")]
#[cfg_attr(not(target_feature = "atomics"), allow(clippy::missing_const_for_fn))]
pub fn park_timeout_ms(
	#[cfg_attr(not(target_feature = "atomics"), allow(unused_variables))] ms: u32,
) {
	#[cfg(target_feature = "atomics")]
	{
		GLOBAL.with(|global| {
			if let Some(Global::Worker(_)) = global {
				park_timeout(Duration::from_millis(ms.into()));
			}
		});
	}
}

/// See [`std::thread::scope()`].
#[track_caller]
pub fn scope<'env, F, T>(#[allow(clippy::min_ident_chars)] f: F) -> T
where
	F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T,
{
	todo!()
}

/// See [`std::thread::sleep()`].
///
/// # Panics
///
/// This call will panic unless called from a thread type that allows blocking,
/// e.g. a Web worker.
pub fn sleep(dur: Duration) {
	#[cfg(target_feature = "atomics")]
	{
		thread::sleep(dur);
	}
	#[cfg(not(target_feature = "atomics"))]
	{
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
}

/// See [`std::thread::sleep_ms()`].
///
/// # Panics
///
/// This call will panic unless called from a thread type that allows blocking,
/// e.g. a Web worker.
#[deprecated(note = "replaced by `web_thread::sleep`")]
pub fn sleep_ms(ms: u32) {
	#[cfg(target_feature = "atomics")]
	{
		#[allow(deprecated)]
		thread::sleep_ms(ms);
	}
	#[cfg(not(target_feature = "atomics"))]
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
