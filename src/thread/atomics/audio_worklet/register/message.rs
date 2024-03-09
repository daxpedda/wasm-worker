//! Handling message related functionality.

use std::cell::OnceCell;

use js_sys::Function;
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};
use web_sys::MessagePort;

use super::super::super::memory::ThreadMemory;
use super::super::super::oneshot::Sender;
use super::super::super::spawn::message::HasMessagePortInterface;
use super::super::super::Thread;

thread_local! {
	pub(in super::super::super) static MESSAGE_PORT: OnceCell<MessagePort> = const { OnceCell::new() };
}

impl HasMessagePortInterface for MessagePort {
	fn set_onmessage(&self, value: Option<&Function>) {
		self.set_onmessage(value);
	}

	fn post_message(&self, message: &JsValue) -> Result<(), JsValue> {
		self.post_message(message)
	}

	fn post_message_with_transfer(
		&self,
		message: &JsValue,
		transfer: &JsValue,
	) -> Result<(), JsValue> {
		self.post_message_with_transferable(message, transfer)
	}
}

/// Data sent to initialize the audio worklet.
#[derive(Debug)]
#[cfg(feature = "message")]
pub(super) struct Data {
	/// [`Thread`].
	pub(super) thread: Thread,
	/// [`Sender`] to send back the associated [`ThreadMemory`].
	pub(super) memory_sender: Sender<ThreadMemory>,
}

/// Register function for the worklet.
///
/// # Safety
///
/// `data` has to be a valid pointer to [`Data`].
#[wasm_bindgen]
#[allow(private_interfaces, unreachable_pub)]
pub unsafe fn __web_thread_worklet_register(data: *mut Data) {
	// SAFETY: Has to be a valid pointer to a `Data`. We only call
	// `__web_thread_worklet_register` from `worklet_with_message.js`. The data sent
	// to it comes only from `RegisterThreadFuture::poll()`.
	let data: Data = *unsafe { Box::from_raw(data) };

	Thread::register(data.thread);
	data.memory_sender.send(ThreadMemory::new());
}
