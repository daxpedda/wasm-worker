use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::Worker;

use super::WorkerOrContext;
use crate::{Message, MessageEvent};

#[derive(Debug)]
pub struct WorkerHandle {
	worker: Worker,
	closure: Option<Closure<dyn FnMut(web_sys::MessageEvent)>>,
}

impl Drop for WorkerHandle {
	fn drop(&mut self) {
		if self.closure.is_some() {
			self.worker.set_onmessage(None);
		}
	}
}

impl WorkerHandle {
	pub(super) fn new(
		worker: Worker,
		closure: Option<Closure<dyn FnMut(web_sys::MessageEvent)>>,
	) -> Self {
		Self { worker, closure }
	}

	#[must_use]
	pub const fn raw(&self) -> &Worker {
		&self.worker
	}

	#[must_use]
	pub fn has_message_handler(&self) -> bool {
		self.closure.is_some()
	}

	pub fn clear_message_handler(&mut self) {
		self.closure.take();
		self.worker.set_onmessage(None);
	}

	pub fn set_message_handler<F: 'static + FnMut(MessageEvent)>(
		&mut self,
		mut message_handler: F,
	) {
		let closure = self.closure.insert(Closure::new(move |event| {
			message_handler(MessageEvent::new(event));
		}));

		self.worker
			.set_onmessage(Some(closure.as_ref().unchecked_ref()));
	}

	pub fn transfer_message<M: IntoIterator<Item = I>, I: Into<Message>>(&self, messages: M) {
		WorkerOrContext::Worker(&self.worker).transfer_messages(messages);
	}

	pub fn terminate(self) {
		self.worker.terminate();
	}
}
