//! TODO

#![allow(unsafe_code)]
// TODO: Temporary.
#![allow(rustdoc::missing_doc_code_examples)]

mod global;
mod worker_url;
#[cfg(feature = "message")]
mod workers;

use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use std::{error, fmt};

use futures_channel::oneshot;
use futures_channel::oneshot::Receiver;
use global::{Global, GLOBAL};
use js_sys::{Array, Function};
use pin_project_lite::pin_project;
use wasm_bindgen::prelude::{wasm_bindgen, Closure};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent, Worker};
use worker_url::WORKER_URL;
#[cfg(feature = "message")]
use workers::{Id, IDS, WORKERS};

/// Handle to the worker.
///
/// [`Poll`](Future::poll)ing [`WorkerHandle`] will return the return value of
/// the associated worker. This depends on established [channels](oneshot) to
/// send back a return value or an error, but Web Workers can be prematurely
/// terminated with
/// [`DedicatedWorkerGlobalScope.close()`](https://developer.mozilla.org/en-US/docs/Web/API/DedicatedWorkerGlobalScope/close).
/// This will cause a memory leak that never drops the [`Sender`] and the
/// [`Future`] will never wake up and [`WorkerHandle`] will never the know that
/// the Worker has finished. Therefore it is recommended to use
/// [`WorkerHandle::terminate()`], which accounts for this possibility.
pub struct WorkerHandle<R> {
	/// ID of the [`Worker`]. We could have stored the [`Worker`] here directly
	/// if we are spawning from the window instead, but this way the
	/// `WorkerHandle` stays [`Send`] and [`Sync`].
	#[cfg(feature = "message")]
	id: Id,
	/// [`Receiver`] to be `await`ed for the return value.
	return_: Option<Return<R>>,
}

/// Holds [`Receiver`] for return value.
struct Return<R>(Receiver<Result<R, Error>>);

impl<R> Debug for WorkerHandle<R> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let mut debug = f.debug_struct("WorkerHandle");
		#[cfg(feature = "message")]
		debug.field("id", &self.id);
		debug.field("return_", &self.return_).finish()
	}
}

impl<R> Debug for Return<R> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_tuple("Running").field(&"Receiver<R>").finish()
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
	/// # Errors
	/// - [`TerminateError::Polled`] if the return value was already received by
	///   [poll](Future::poll)ing.
	/// - [`TerminateError::Error`] if the worker panicked or was already
	///   terminated.
	///
	/// # Safety
	/// This function will leak the workers stack and TLS. This function is not
	/// marked as unsafe because Rust's safety guarantees don't cover leaking
	/// memory or guaranteeing that destructors will run.
	///
	/// # Panics
	/// Panics if trying to call from anything else than the window or a
	/// dedicated worker.
	#[cfg(feature = "message")]
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
			Global::Worker(worker) => worker.post_message_ext(Message::Terminate(self.id)),
		});

		result
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
/// automatically terminate itself when the closure has finished running.
///
/// Nested workers terminate when the worker they are spawned from is closed,
/// they are not supported. To be more aligned with Rust's [`std::thread`]
/// spawning a worker in a worker will instead spawn the worker in the window,
/// therefore finishing the worker it spawned from will not terminate
/// the newly spawned worker.
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

	#[cfg(feature = "message")]
	let id = IDS.next();
	let context = WorkerContext::Closure(Box::new(move || {
		let result = try_(f).map_err(Error::Error);
		let _canceled = sender.send(result);

		#[cfg(feature = "message")]
		GLOBAL.with(|global| global.worker().post_message_ext(Message::Close(id)));
	}));

	spawn_internal(
		#[cfg(feature = "message")]
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

	#[cfg(feature = "message")]
	let id = IDS.next();
	let worker = WorkerContext::Future(Box::new(move || {
		Box::pin(async move {
			let result = TryFuture::new(f()).await.map_err(Error::Error);
			let _canceled = sender.send(result);

			#[cfg(feature = "message")]
			GLOBAL.with(|global| global.worker().post_message_ext(Message::Close(id)));
		})
	}));

	spawn_internal(
		#[cfg(feature = "message")]
		id,
		worker,
		receiver,
	)
}

/// Holds the functions to execute on the worker.
enum WorkerContext {
	/// Closure.
	Closure(Box<dyn 'static + FnOnce() + Send>),
	/// [`Future`].
	Future(Box<dyn 'static + FnOnce() -> Pin<Box<dyn 'static + Future<Output = ()>>> + Send>),
}

/// Message sent to the window.
enum Message {
	/// Instruct window to spawn a worker.
	Spawn {
		/// ID to use for the spawned worker.
		#[cfg(feature = "message")]
		id: Id,
		/// Worker context to run.
		context: WorkerContext,
	},
	/// Instruct window to terminate a worker.
	#[cfg(feature = "message")]
	Terminate(Id),
	/// Instruct window to delete this [`Worker`] from the
	/// [`Workers`](workers::Workers) list.
	#[cfg(feature = "message")]
	Close(Id),
}

/// Internal worker spawning function.
fn spawn_internal<R>(
	#[cfg(feature = "message")] id: Id,
	context: WorkerContext,
	receiver: Receiver<Result<R, Error>>,
) -> WorkerHandle<R> {
	GLOBAL.with(|global| match global {
		Global::Window => spawn_from_window(
			#[cfg(feature = "message")]
			id,
			context,
		),
		Global::Worker(global) => spawn_from_worker(
			global,
			#[cfg(feature = "message")]
			id,
			context,
		),
	});

	WorkerHandle {
		#[cfg(feature = "message")]
		id,
		return_: Some(Return(receiver)),
	}
}

/// Spawn worker from window.
fn spawn_from_window(#[cfg(feature = "message")] id: Id, context: WorkerContext) {
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

	#[cfg(feature = "message")]
	WORKERS
		.with(|workers| workers.push(id, worker))
		.expect("duplicate ID used");
}

/// Spawn worker from worker.
fn spawn_from_worker(
	global: &DedicatedWorkerGlobalScope,
	#[cfg(feature = "message")] id: Id,
	context: WorkerContext,
) {
	global.post_message_ext(Message::Spawn {
		#[cfg(feature = "message")]
		id,
		context,
	});
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

/// Wrap a function in a JS `try catch` block.
fn try_<R>(fn_: impl FnOnce() -> R) -> Result<R, String> {
	#[wasm_bindgen]
	extern "C" {
		/// JS `try catch` block.
		fn __wasm_worker_try(fn_: &mut dyn FnMut()) -> JsValue;
	}

	// This workaround is required because of the limitations of having to pass an
	// `FnMut`, `FnOnce` isn't supported by `wasm_bindgen`.
	let mut fn_ = Some(fn_);
	let mut return_ = None;
	let error =
		__wasm_worker_try(&mut || return_ = Some(fn_.take().expect("called more than once")()));
	return_.ok_or(format!("{error:?}"))
}

pin_project! {
	/// Wrapping a [`Future`] in a JS `try catch` block.
	pub struct TryFuture<F: Future>{
		#[pin]
		fn_: F
	}
}

impl<F> Future for TryFuture<F>
where
	F: Future,
{
	type Output = Result<F::Output, String>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		match try_(|| self.project().fn_.poll(cx)) {
			Ok(Poll::Ready(return_)) => Poll::Ready(Ok(return_)),
			Ok(Poll::Pending) => Poll::Pending,
			Err(err) => Poll::Ready(Err(err)),
		}
	}
}

impl<F: Future> TryFuture<F> {
	/// Creates a new [`TryFuture`].
	const fn new(fn_: F) -> Self {
		Self { fn_ }
	}
}

/// Convenience methods for [`DedicatedWorkerGlobalScope`].
trait WorkerExt {
	/// Handle turning [`Message`] into a pointer and cleaning it up in case of
	/// an error.
	fn post_message_ext(&self, message: Message);
}

impl WorkerExt for DedicatedWorkerGlobalScope {
	fn post_message_ext(&self, message: Message) {
		let message = Box::into_raw(Box::new(message));

		if let Err(error) = self.post_message(
			#[allow(clippy::as_conversions)]
			&f64::from_bits(message as u64).into(),
		) {
			// SAFETY: We created this pointer just above. This is necessary to clean up
			// memory in the case of an error.
			drop(unsafe { Box::from_raw(message) });
			// `Worker.postMessage()` should only fail on unsupported messages, this is
			// consistent and is caught during testing.
			unreachable!("`Worker.postMessage()` failed: {error:?}");
		}
	}
}

thread_local! {
	/// All workers are spawned from the window only, so having this thread-local is enough.
	static MESSAGE_HANDLER: MessageHandler = MessageHandler::new();
}

/// Holds the callback to handle nested worker spawning.
struct MessageHandler(Closure<dyn FnMut(&MessageEvent)>);

impl Deref for MessageHandler {
	type Target = Function;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().unchecked_ref()
	}
}

impl MessageHandler {
	/// Creates a [`MessageHandler`].
	fn new() -> Self {
		// We don't need to worry about the deallocation of this `Closure`, we only
		// generate it once for every worker and store it in a thread-local, Rust will
		// then deallocate it for us.
		Self(Closure::wrap(Box::new(|event: &MessageEvent| {
			// We reconstruct the pointer address from the bits stored as a `f64` in a
			// `JsValue`.
			#[allow(clippy::as_conversions)]
			let message = event.data().as_f64().expect("expected `f64`").to_bits() as *mut Message;
			// SAFETY: We created this pointer in `spawn_from_worker()`.
			let message = *unsafe { Box::from_raw(message) };

			match message {
				Message::Spawn {
					#[cfg(feature = "message")]
					id,
					context,
				} => spawn_from_window(
					#[cfg(feature = "message")]
					id,
					context,
				),
				#[cfg(feature = "message")]
				Message::Terminate(id) => {
					WORKERS.with(|workers| {
						if let Some(worker) = workers.remove(id) {
							worker.terminate();
						}
					});
				}
				#[cfg(feature = "message")]
				Message::Close(id) => {
					WORKERS.with(|workers| {
						if workers.remove(id).is_none() {
							web_sys::console::warn_1(&"unknown worker ID closed".into());
						}
					});
				}
			}
		})))
	}
}
