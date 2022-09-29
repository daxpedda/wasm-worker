//! TODO
//!
//! # Implementation Details
//!
//! ## Un-nested Workers
//!
//! The Web API creates a nested worker when you spawn a worker from a worker. A
//! nested worker will terminate if the parent worker is closed.
//!
//! This does not align with Rust's [`std::thread`], therefor when using
//! [`spawn()`] or [`spawn_async()`] from inside a worker, the new worker will
//! be spawned from the window instead, preventing spawning nested workers
//! altogether.
//!
//! ## Automatic Worker Closing
//!
//! When [spawning](spawn()) a worker, the given closure or [`Future`] is
//! executed. After it is done, the worker will automatically close, as is done
//! with Rust's [`std::thread`] implementation.
//!
//! This can be confusing if the background tasks were spawned, like a
//! [`Promise`], that are expected to finish: as the worker will close, all
//! [`Promise`]s will be aborted. To prevent this, [`Promise`]s can be `await`ed
//! at the end of the given task.
//!
//! This is not considered an [issue](#issues) because the async runtime in the
//! Web API is not multi-threaded and therefor this behavior is similar to how
//! Rust native single-threaded async runtimes behave when the thread they are
//! used in is finished.
//!
//! # Issues
//!
//! ## Messaging
//!
//! It is possible to overwrite a workers message handler by using
//! [`DedicatedWorkerGlobalScope.onmessage()`]. Apart from not being useful, as
//! the [`Worker`] object is not accessible and therefor
//! [`Worker.postMessage()`] can't be called, it will also break any
//! functionality provided by the `track` crate feature.
//!
//! [`DedicatedWorkerGlobalScope.postMessage()`] should also not be used, as the
//! [`Worker.onmessage()`] handler is used for spawning workers from workers and
//! functionality provided by the `track` crate feature.
//!
//! If you need to send JS values between workers, see [`send()`] or
//! [`WorkerHandle::send()`].
//!
//! ## External Termination
//!
//! It is possible to terminate a worker by calling
//! [`DedicatedWorkerGlobalScope.close()`]. This can cause multiple problems:
//! - The corresponding [`WorkerHandle`] won't wake up when
//!   [`poll`](Future::poll)ed.
//! - The corresponding [`WorkerHandle::try_join()`] will always return
//!   [`None`].
//! - The corresponding [`WorkerHandle::is_terminated()`] will always return
//!   [`false`].
//! - The stack and TLS of this worker will be leaked.
//!
//! If you really need this functionality use [`terminate()`] or
//! [`WorkerHandle::terminate()`].
//!
//! ## Internal Termination
//!
//! Using [`terminate()`] or [`WorkerHandle::terminate()`] solve the problems
//! outlined in [#External Termination](#external-termination) but in addition
//! to [not running TLS destructors](#tls-destructors) the stack is not unwound
//! and therefor destructors of objects in the running task are not called.
//!
//! Only destructors aren't called, the stack is not leaked.
//!
//! Generally speaking, worker termination is not recommended unless you are
//! trying to close the application.
//!
//! ## TLS Destructors
//!
//! TLS destructors in workers are never called. A way to avoid this issue is to
//! never close workers by letting it's task never finish but to keep re-using
//! them or to be careful not to use objects in TLS that have destructors.
//!
//! ## Panic Behavior
//!
//! The `wasm32-unknown-unknown` target can only be used with `panic = "abort"`,
//! as Rust has no support for WASM exception handling. The expected Rust
//! behavior is that on panicking the application should abort; the Web API has
//! no support for aborting an instantiated WASM module.
//!
//! [wasm-worker](crate) will simply catch the panic and send it to the
//! corresponding [`WorkerHandle`]. The worker will close, but the window will
//! not. As a workaround one could terminate all workers and interrupt the main
//! loop, keep in mind that spawned background tasks in the window, like a
//! [`Promise`], will still continue running. Afterwards the browser is
//! responsible for garbage-collecting the WASM module.
//!
//! This behavior also applies, but is not limited, to [`throw`],
//! [`std::process::abort()`] and [`std::arch::wasm32::unreachable()`].
//!
//! [`DedicatedWorkerGlobalScope.close()`]: https://developer.mozilla.org/en-US/docs/Web/API/DedicatedWorkerGlobalScope/close
//! [`DedicatedWorkerGlobalScope.onmessage()`]: https://developer.mozilla.org/en-US/docs/Web/API/DedicatedWorkerGlobalScope/message_event
//! [`DedicatedWorkerGlobalScope.postMessage()`]: https://developer.mozilla.org/en-US/docs/Web/API/DedicatedWorkerGlobalScope/postMessage
//! [`Promise`]: js_sys::Promise
//! [`throw`]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/throw
//! [`Worker.onmessage()`]: https://developer.mozilla.org/en-US/docs/Web/API/Worker/message_event
//! [`Worker.postMessage()`]: https://developer.mozilla.org/en-US/docs/Web/API/Worker/postMessage

#![allow(unsafe_code)]

mod global;
mod message_handler;
mod try_catch;
mod worker_url;
#[cfg(feature = "track")]
mod workers;

use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::panic::PanicInfo;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use std::{error, fmt};

use futures_channel::oneshot;
use futures_channel::oneshot::Receiver;
use global::{Global, GLOBAL};
use js_sys::Array;
use message_handler::{Message, WorkerContext, MESSAGE_HANDLER};
use try_catch::TryFuture;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::{DedicatedWorkerGlobalScope, Worker};
use worker_url::WORKER_URL;
#[cfg(feature = "track")]
use workers::{Id, IDS, WORKERS};

/// Handle to the worker.
///
/// See [#External Closing](#external-closing).
pub struct WorkerHandle<R> {
	/// ID of the [`Worker`]. We could have stored the [`Worker`] here directly
	/// if we are spawning from the window instead, but this way the
	/// `WorkerHandle` stays [`Send`] and [`Sync`].
	#[cfg(feature = "track")]
	id: Id,
	/// [`Receiver`] to be `await`ed for the return value.
	return_: Option<Return<R>>,
}

/// Holds the [`Receiver`] for return value.
struct Return<R>(Receiver<Result<R, Error>>);

impl<R> Debug for WorkerHandle<R> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let mut debug = f.debug_struct("WorkerHandle");
		#[cfg(feature = "track")]
		debug.field("id", &self.id);
		debug.field("return_", &self.return_).finish()
	}
}

impl<R> Future for WorkerHandle<R> {
	type Output = Result<R, Error>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let return_ = self.return_.as_mut().expect("polled after completion");
		let result = ready!(Pin::new(&mut return_.0).poll(cx)).unwrap_or(Err(Error::Terminated));

		self.return_.take();

		Poll::Ready(result)
	}
}

impl<R> WorkerHandle<R> {
	/// Terminates the spawned worker. This does not offer the worker an
	/// opportunity to finish its operations; it is stopped at once.
	///
	/// If the worker was already done, this will return the return value or the
	/// error, otherwise will return [`None`].
	///
	/// See [#TLS Destructors](#tls-destructors).
	///
	/// # Errors
	/// - [`TerminateError::Polled`] if the return value was already received by
	///   [poll](Future::poll)ing.
	/// - [`TerminateError::Error`] if the worker panicked or was already
	///   terminated.
	///
	/// # Panics
	/// Panics if trying to call from anything else than the window or a
	/// dedicated worker.
	#[cfg(feature = "track")]
	pub fn terminate(self) -> Result<Option<R>, TerminateError> {
		let result = self
			.return_
			.ok_or(TerminateError::Polled)
			.and_then(|Return(mut return_)| match return_.try_recv() {
				Ok(Some(Ok(ok))) => Ok(Some(ok)),
				Ok(Some(Err(error))) => Err(TerminateError::Error(error)),
				Ok(None) => Ok(None),
				Err(_) => Err(TerminateError::Error(Error::Terminated)),
			});

		GLOBAL.with(|global| match global {
			// The window has access to all workers, we can terminate right here.
			Global::Window => {
				WORKERS.with(|workers| {
					if let Some(worker) = workers.remove(self.id) {
						worker.terminate();
					}
				});
			}
			// Workers have to instruct the window to terminate the worker for them.
			Global::Worker(worker) => Message::Terminate(self.id).post_message(worker),
		});

		result
	}
}

impl<R> Debug for Return<R> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_tuple("Return")
			.field(&"Receiver<Result<R, Error>>")
			.finish()
	}
}

/// [`Error`](error::Error) returned when the worker panicked or was terminated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
	/// Worker panicked or an error was [thrown](wasm_bindgen::throw_val).
	Error(String),
	/// Worker was terminated.
	Terminated,
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Error(error) => write!(f, "{error:?}"),
			Self::Terminated => write!(f, "worker terminated before receiving a return value"),
		}
	}
}

impl error::Error for Error {}

impl From<Error> for JsValue {
	fn from(error: Error) -> Self {
		error.to_string().into()
	}
}

/// [`Error`](error::Error) returned when terminating a worker through
/// [`WorkerHandle::terminate()`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminateError {
	/// Return value was already received by [`poll`](Future::poll)ing.
	Polled,
	/// See [`Error`].
	Error(Error),
}

impl Display for TerminateError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Polled => write!(f, "worker return value was already received by polling"),
			Self::Error(error) => write!(f, "{error}"),
		}
	}
}

impl error::Error for TerminateError {}

impl From<TerminateError> for JsValue {
	fn from(error: TerminateError) -> Self {
		error.to_string().into()
	}
}

/// Spawn a new worker and run the given closure in it. The worker will
/// automatically close itself when the closure has finished running.
///
/// To be more aligned with Rust's [`std::thread`], spawning a worker in a
/// worker will instead spawn the worker from the window. This in contrast to
/// how nested workers behave in the browser: closing the worker it spawned from
/// will not terminate the newly spawned worker.
///
/// # Panics
/// Panics if trying to spawn from anything else than the window or a dedicated
/// worker.
pub fn spawn<F, R>(f: F) -> WorkerHandle<R>
where
	F: 'static + FnOnce() -> R + Send,
	R: 'static + Send,
{
	let (sender, receiver) = oneshot::channel();

	#[cfg(feature = "track")]
	let id = IDS.next();
	let context = WorkerContext::Closure(Box::new(move || {
		let result = try_catch::try_(f).map_err(Error::Error);
		let _canceled = sender.send(result);

		#[cfg(feature = "track")]
		GLOBAL.with(|global| Message::Close(id).post_message(global.worker()));
	}));

	spawn_internal(
		#[cfg(feature = "track")]
		id,
		context,
		receiver,
	)
}

/// Spawn a new worker and run the given [`Future`] in it. The parameter
/// expected is a closure that returns a [`Future`], this is designed to ensure
/// that only the closure requires [`Send`], but the [`Future`] does not.
///
/// For more details see [`spawn()`].
///
/// # Panics
/// Panics if trying to spawn from anything else than the window or a dedicated
/// worker.
pub fn spawn_async<F1, F2, R>(f: F1) -> WorkerHandle<R>
where
	F1: 'static + FnOnce() -> F2 + Send,
	F2: 'static + Future<Output = R>,
	R: 'static + Send,
{
	let (sender, receiver) = oneshot::channel();

	#[cfg(feature = "track")]
	let id = IDS.next();
	let worker = WorkerContext::Future(Box::new(move || {
		Box::pin(async move {
			// Try to catch panics in the user-given closure that produces the `Future`.
			let result = match try_catch::try_(f).map_err(Error::Error) {
				Ok(f) => TryFuture::new(f).await.map_err(Error::Error),
				Err(error) => Err(error),
			};
			let _canceled = sender.send(result);

			#[cfg(feature = "track")]
			GLOBAL.with(|global| Message::Close(id).post_message(global.worker()));
		})
	}));

	spawn_internal(
		#[cfg(feature = "track")]
		id,
		worker,
		receiver,
	)
}

/// Hook for panic handler. Ensures that instead of just using a trap on panic
/// we also throw the panic message, which will be caught and relayed to the
/// [`WorkerHandle`].
pub fn hook(panic_info: &PanicInfo<'_>) -> ! {
	wasm_bindgen::throw_str(&panic_info.to_string());
}

/// Internal worker spawning function.
fn spawn_internal<R>(
	#[cfg(feature = "track")] id: Id,
	context: WorkerContext,
	receiver: Receiver<Result<R, Error>>,
) -> WorkerHandle<R> {
	GLOBAL.with(|global| match global {
		Global::Window => spawn_from_window(
			#[cfg(feature = "track")]
			id,
			context,
		),
		Global::Worker(global) => spawn_from_worker(
			global,
			#[cfg(feature = "track")]
			id,
			context,
		),
	});

	WorkerHandle {
		#[cfg(feature = "track")]
		id,
		return_: Some(Return(receiver)),
	}
}

/// Spawn worker from window.
fn spawn_from_window(#[cfg(feature = "track")] id: Id, context: WorkerContext) {
	// `Worker.new()` should only fail on unsupported `URL`s, this is consistent,
	// except the `wasm_bindgen::script_url()` determined during run-time and part
	// of the `WORKER_URL`, and is caught during testing.
	let worker = WORKER_URL
		.with(|worker_url| Worker::new(worker_url))
		.expect("`Worker.new()` failed");

	MESSAGE_HANDLER.with(|callback| worker.set_onmessage(Some(callback)));

	let context = Box::into_raw(Box::new(context));

	let init = Array::of3(
		&wasm_bindgen::module(),
		&wasm_bindgen::memory(),
		#[allow(clippy::as_conversions)]
		&(context as usize).into(),
	);

	if let Err(error) = worker.post_message(&init) {
		// SAFETY: We created this pointer just above. This is necessary to clean up
		// memory in the case of an error.
		drop(unsafe { Box::from_raw(context) });
		unreachable!("`Worker.postMessage()` failed: {error:?}")
	}

	#[cfg(feature = "track")]
	WORKERS
		.with(|workers| workers.push(id, worker))
		.expect("duplicate ID used");
}

/// Spawn worker from worker.
fn spawn_from_worker(
	global: &DedicatedWorkerGlobalScope,
	#[cfg(feature = "track")] id: Id,
	context: WorkerContext,
) {
	Message::Spawn {
		#[cfg(feature = "track")]
		id,
		context,
	}
	.post_message(global);
}

/// This function is called to get back into the Rust module from inside the
/// spawned worker.
#[doc(hidden)]
#[allow(clippy::future_not_send)]
#[wasm_bindgen]
pub async fn __wasm_worker_entry(context: usize) {
	#[allow(clippy::as_conversions)]
	// SAFETY: The argument is an address that has to be a valid pointer to a `WorkerContext`.
	match *unsafe { Box::from_raw(context as *mut WorkerContext) } {
		WorkerContext::Closure(fn_) => fn_(),
		WorkerContext::Future(fn_) => fn_().await,
	};
}
