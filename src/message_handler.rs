//! Handling of [`Message`]s send with
//! [`Worker::post_message()`](web_sys::Worker::post_message).

use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;

use js_sys::Function;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

#[cfg(feature = "track")]
use crate::workers::{Id, WORKERS};

/// Message sent to the window.
pub(crate) enum Message {
	/// Instruct window to spawn a worker.
	Spawn {
		/// ID to use for the spawned worker.
		#[cfg(feature = "track")]
		id: Id,
		/// Worker context to run.
		context: WorkerContext,
	},
	/// Instruct window to terminate a worker.
	#[cfg(feature = "track")]
	Terminate(Id),
	/// Instruct window to delete this [`Worker`][web_sys::Worker] from the
	/// [`Workers`](crate::workers::Workers) list.
	#[cfg(feature = "track")]
	Close(Id),
}

/// Holds the functions to execute on the worker.
pub(crate) enum WorkerContext {
	/// Closure.
	Closure(Box<dyn 'static + FnOnce() + Send>),
	/// [`Future`].
	Future(Box<dyn 'static + FnOnce() -> Pin<Box<dyn 'static + Future<Output = ()>>> + Send>),
}

impl Message {
	/// Handle turning [`Message`] into a pointer and cleaning it up in case of
	/// an error.
	pub(crate) fn post_message(self, global: &DedicatedWorkerGlobalScope) {
		let message = Box::into_raw(Box::new(self));

		if let Err(error) = global.post_message(
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
	pub(crate) static MESSAGE_HANDLER: MessageHandler = MessageHandler::new();
}

/// Holds the callback to handle nested worker spawning.
pub(crate) struct MessageHandler(Closure<dyn FnMut(&MessageEvent)>);

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
					#[cfg(feature = "track")]
					id,
					context,
				} => crate::spawn_from_window(
					#[cfg(feature = "track")]
					id,
					context,
				),
				#[cfg(feature = "track")]
				Message::Terminate(id) => {
					WORKERS.with(|workers| {
						if let Some(worker) = workers.remove(id) {
							worker.terminate();
						}
					});
				}
				#[cfg(feature = "track")]
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
