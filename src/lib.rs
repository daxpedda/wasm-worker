//! TODO

#![allow(unsafe_code)]

use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_channel::oneshot;
use futures_channel::oneshot::Receiver;
use js_sys::{Array, Function};
use wasm_bindgen::prelude::{wasm_bindgen, Closure};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
	console, Blob, BlobPropertyBag, DedicatedWorkerGlobalScope, MessageEvent, Url, Worker,
};

/// Holds the functions to execute on the worker.
enum WorkerContext {
	/// Closure.
	Closure(Box<dyn 'static + FnOnce() + Send>),
	/// [`Future`].
	Future(Box<dyn 'static + FnOnce() -> Pin<Box<dyn 'static + Future<Output = ()>>> + Send>),
}

/// Can be `await`ed to get the return value.
#[derive(Debug)]
pub struct JoinHandle<R>(Receiver<R>);

/// [`Error`] returned when the worker was terminated or canceled prematurely.
#[derive(Clone, Copy, Debug)]
pub struct Canceled;

impl Display for Canceled {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "worker canceled before receiving a return value")
	}
}

impl Error for Canceled {}

impl<R> Future for JoinHandle<R> {
	type Output = Result<R, Canceled>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Pin::new(&mut self.0).poll(cx).map_err(|_| Canceled)
	}
}

/// Spawn a new worker and run the given closure in it. The worker will
/// automatically terminate itself when the closure has finished running.
///
/// Nested workers terminate when the worker they are spawned from is closed,
/// they are not supported. To be more aligned with Rust's [`std::thread`]
/// spawning a worker in a worker will instead spawn the worker in the main
/// thread, therefore finishing the worker it spawned from will not terminate
/// the newly spawned worker.
pub fn spawn<F, R>(f: F) -> JoinHandle<R>
where
	F: 'static + FnOnce() -> R + Send,
	R: 'static + Send,
{
	let (sender, receiver) = oneshot::channel();
	spawn_internal(WorkerContext::Closure(Box::new(|| {
		let _canceled = sender.send(f());
	})));
	JoinHandle(receiver)
}

/// Spawn a new worker and run the given [`Future`] in it. The parameter
/// expected is a closure that returns a [`Future`], this is done to ensure that
/// only the closure requires [`Send`], but the [`Future`] does not.
///
/// For more details see [`spawn()`].
pub fn spawn_async<F1, F2, R>(f: F1) -> JoinHandle<R>
where
	F1: 'static + FnOnce() -> F2 + Send,
	F2: 'static + Future<Output = R>,
	R: 'static + Send,
{
	let (sender, receiver) = oneshot::channel();
	spawn_internal(WorkerContext::Future(Box::new(|| {
		Box::pin(async {
			let _canceled = sender.send(f().await);
		})
	})));
	JoinHandle(receiver)
}

/// Internal worker spawning function. Only converts [`WorkerContext`] to a
/// pointer.
fn spawn_internal(context: WorkerContext) {
	spawn_internal_ptr(Box::into_raw(Box::new(context)));
}

/// Internal worker spawning function. Takes a pointer to [`WorkerContext`].
fn spawn_internal_ptr(context: *mut WorkerContext) {
	// Don't leak memory, on error clean up before panicking.
	if let Err(err) = spawn_internal_error(context) {
		console::log_1(&"test".into());
		// SAFETY: The pointer passed in as an argument has to be valid. If this
		// function is called by `spawn_internal()` then everything is fine.
		drop(unsafe { Box::from_raw(context) });
		panic!("{err:?}");
	}
}

/// Internal worker spawning function. Returns [`Result`] to prevent memory
/// leaks.
fn spawn_internal_error(context: *mut WorkerContext) -> Result<(), JsValue> {
	WINDOW_OR_WORKER.with(|global| {
		let global = global.as_ref().map_err(|error| JsValue::from(*error))?;

		match global {
			// Spawn the worker from the main thread.
			WindowOrWorker::Window => {
				let worker = WORKER_URL.with(|worker_url| {
					let worker_url = worker_url.as_ref().map_err(JsValue::from)?;
					Worker::new(worker_url)
				})?;

				NESTED_WORKER.with(|callback| worker.set_onmessage(Some(callback)));

				let init = Array::of3(
					&wasm_bindgen::module(),
					&wasm_bindgen::memory(),
					#[allow(clippy::as_conversions)]
					&(context as usize).into(),
				);

				worker.post_message(&init)
			}
			// When inside a worker, tell main thread to spawn worker for us.
			WindowOrWorker::Worker(worker) => {
				// `JsValue` numbers are always `f64`. Using `JsValue::from()` will actually do
				// numerical conversions. But we will just store the bits as a `f64` and
				// reconstruct the pointer address from bits again.
				worker.post_message(
					#[allow(clippy::as_conversions)]
					&f64::from_bits(context as u64).into(),
				)
			}
		}
	})
}

/// This function is called to get back into the Rust module from inside the
/// spawned worker.
#[doc(hidden)]
#[allow(clippy::future_not_send)]
#[wasm_bindgen]
pub async fn __wasm_thread_entry(context: usize) {
	#[allow(clippy::as_conversions)]
	// SAFETY: The argument is an address that has to be a valid pointer to a `WorkerContext`.
	match *unsafe { Box::from_raw(context as *mut WorkerContext) } {
		WorkerContext::Closure(f) => f(),
		WorkerContext::Future(f) => f().await,
	}
}

thread_local! {
	/// Can be re-used by all workers. All workers are spawned from the main thread only, so having
	/// this thread-local is enough.
	static WORKER_URL: Result<WorkerUrl, String> = WorkerUrl::new();
}

/// Holds the worker [`Url`]. This is important so it's deallocated when we are
/// done.
struct WorkerUrl(String);

impl Deref for WorkerUrl {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Drop for WorkerUrl {
	fn drop(&mut self) {
		if let Err(error) = Url::revoke_object_url(&self.0) {
			console::warn_1(&format!("worker `Url` could not be deallocated: {error:?}").into());
		}
	}
}

impl WorkerUrl {
	/// Creates a new [`WorkerUrl`]. We return a [`Result`] to get a chance to
	/// clean up pointers before we panic.
	fn new() -> Result<Self, String> {
		// Put together the JS shim running on the worker.
		// Import the JS shim generated by `wasm-bindgen`.
		let header = format!("importScripts('{}');\n", wasm_bindgen::script_url());
		// Add the worker JS shim, receiving the WASM module and starting the worker.
		let script = include_str!("web_worker.js");

		// Create the `Blob` necessary to create the worker `Url`.
		let sequence = Array::of2(&JsValue::from(header.as_str()), &JsValue::from(script));
		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property);

		// Create the `Url` for the worker.
		let url = blob
			.and_then(|blob| Url::create_object_url_with_blob(&blob))
			.map_err(|error| format!("worker `Url` could not be created: {error:?}"))?;

		Ok(Self(url))
	}
}

thread_local! {
	/// Can be re-used by all workers. All workers are spawned from the main thread only, so having
	/// this thread-local is enough. De-allocation is handled by Rust.
	static NESTED_WORKER: NestedWorker = NestedWorker::new();
}

/// Holds the callback to handle nested worker spawning.
struct NestedWorker(Closure<dyn FnMut(&MessageEvent)>);

impl Deref for NestedWorker {
	type Target = Function;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().unchecked_ref()
	}
}

impl NestedWorker {
	/// Creates a [`NestedWorker`].
	fn new() -> Self {
		// We don't need to worry about the de-allocation of this `Closure`, we only
		// generate it once for every worker and store it in a thread-local, Rust will
		// then de-allocate it for us.
		Self(Closure::wrap(Box::new(|event: &MessageEvent| {
			// We reconstruct the pointer address from the bits stored as a `f64`.
			#[allow(clippy::as_conversions)]
			let context = event.data().as_f64().expect("expected `f64`").to_bits() as *mut WorkerContext;
			spawn_internal_ptr(context);
		})))
	}
}

thread_local! {
	static WINDOW_OR_WORKER: Result<WindowOrWorker, &'static str> = WindowOrWorker::new();
}

/// This helps us determine if we are in a worker or in the main thread.
enum WindowOrWorker {
	/// Main thread.
	Window,
	/// Worker.
	Worker(DedicatedWorkerGlobalScope),
}

impl WindowOrWorker {
	/// Creates a [`WindowOrWorker`]. We return a [`Result`] to get a chance to
	/// clean up pointers before we panic.
	// TODO: Clippy false-positive.
	// See <https://github.com/rust-lang/rust-clippy/issues/6902>.
	#[allow(clippy::use_self)]
	fn new() -> Result<WindowOrWorker, &'static str> {
		// We need this to detect the context we are in without getting JS parsing
		// errors from the generated JS shim by `wasm-bindgen`.
		#[wasm_bindgen]
		extern "C" {
			type Global;

			#[wasm_bindgen(method, getter, js_name = Window)]
			fn window(this: &Global) -> JsValue;

			#[wasm_bindgen(method, getter, js_name = DedicatedWorkerGlobalScope)]
			fn worker(this: &Global) -> JsValue;
		}

		let global: Global = js_sys::global().unchecked_into();

		if !global.window().is_undefined() {
			Ok(Self::Window)
		} else if !global.worker().is_undefined() {
			Ok(Self::Worker(global.unchecked_into()))
		} else {
			Err("only supported in a browser or web worker")
		}
	}
}
