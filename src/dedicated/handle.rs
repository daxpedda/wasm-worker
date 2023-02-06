use std::cell::RefCell;
use std::rc::{Rc, Weak};

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::Worker;

use super::{MessageClosure, WorkerOrContext};
use crate::{Message, MessageEvent};

#[derive(Debug)]
pub struct WorkerHandle {
	worker: Worker,
	message_handler: MessageHandler,
}

#[derive(Debug)]
enum MessageHandler {
	Strong(Rc<RefCell<MessageClosure>>),
	Weak(Weak<RefCell<MessageClosure>>),
}

impl Drop for WorkerHandle {
	fn drop(&mut self) {
		if let MessageHandler::Strong(message_handler) = &self.message_handler {
			if Rc::strong_count(message_handler) == 1 {
				self.worker.set_onmessage(None);
			}
		}
	}
}

impl WorkerHandle {
	pub(super) fn new(worker: Worker, message_handler: Rc<RefCell<MessageClosure>>) -> Self {
		Self {
			worker,
			message_handler: MessageHandler::Strong(message_handler),
		}
	}

	pub(super) fn new_weak(worker: Worker, message_handler: Weak<RefCell<MessageClosure>>) -> Self {
		Self {
			worker,
			message_handler: MessageHandler::Weak(message_handler),
		}
	}

	#[must_use]
	pub const fn raw(&self) -> &Worker {
		&self.worker
	}

	fn with_message_handler<T>(&self, f: impl FnOnce(&MessageClosure) -> T) -> T {
		match &self.message_handler {
			MessageHandler::Strong(rc) => f(&RefCell::borrow(rc)),
			MessageHandler::Weak(weak) => {
				f(&mut RefCell::borrow_mut(&Weak::upgrade(weak).unwrap()))
			}
		}
	}

	fn with_message_handler_mut<T>(&self, f: impl FnOnce(&mut MessageClosure) -> T) -> T {
		match &self.message_handler {
			MessageHandler::Strong(rc) => f(&mut RefCell::borrow_mut(rc)),
			MessageHandler::Weak(weak) => {
				f(&mut RefCell::borrow_mut(&Weak::upgrade(weak).unwrap()))
			}
		}
	}

	#[must_use]
	pub fn has_message_handler(&self) -> bool {
		self.with_message_handler(Option::is_some)
	}

	pub fn clear_message_handler(&mut self) {
		self.with_message_handler_mut(Option::take);
		self.worker.set_onmessage(None);
	}

	pub fn set_message_handler<F: 'static + FnMut(&Self, MessageEvent)>(
		&mut self,
		mut new_message_handler: F,
	) {
		let handle = Self {
			worker: self.worker.clone(),
			message_handler: MessageHandler::Weak(match &self.message_handler {
				MessageHandler::Strong(message_handler) => Rc::downgrade(message_handler),
				MessageHandler::Weak(message_handler) => Weak::clone(message_handler),
			}),
		};

		self.with_message_handler_mut(|message_handler| {
			let message_handler = message_handler.insert(Closure::new(move |event| {
				new_message_handler(&handle, MessageEvent::new(event));
			}));

			self.worker
				.set_onmessage(Some(message_handler.as_ref().unchecked_ref()));
		});
	}

	pub fn transfer_message<M: IntoIterator<Item = I>, I: Into<Message>>(&self, messages: M) {
		WorkerOrContext::Worker(&self.worker).transfer_messages(messages);
	}

	pub fn terminate(self) {
		self.worker.terminate();
	}
}
