use std::cell::RefCell;
use std::future::Future;
use std::rc::{Rc, Weak};

use web_sys::Worker;

use super::{Closure, OldMessageHandler, WorkerOrContext};
use crate::{Message, MessageEvent};

#[derive(Clone, Debug)]
pub struct WorkerHandle {
	worker: Worker,
	message_handler: Rc<RefCell<Option<Closure>>>,
}

impl Drop for WorkerHandle {
	fn drop(&mut self) {
		if Rc::strong_count(&self.message_handler) == 1 {
			self.worker.set_onmessage(None);
		}
	}
}

impl Eq for WorkerHandle {}

impl PartialEq for WorkerHandle {
	fn eq(&self, other: &Self) -> bool {
		self.worker == other.worker
	}
}

impl WorkerHandle {
	pub(super) fn new(worker: Worker, message_handler: Rc<RefCell<Option<Closure>>>) -> Self {
		Self {
			worker,
			message_handler,
		}
	}

	#[must_use]
	pub const fn raw(&self) -> &Worker {
		&self.worker
	}

	#[must_use]
	pub fn has_message_handler(&self) -> bool {
		RefCell::borrow(&self.message_handler).is_some()
	}

	#[allow(clippy::must_use_candidate)]
	pub fn clear_message_handler(&self) -> OldMessageHandler {
		let old_message_handler = RefCell::borrow_mut(&self.message_handler).take();
		self.worker.set_onmessage(None);

		OldMessageHandler::new(old_message_handler)
	}

	pub fn set_message_handler<F: 'static + FnMut(&WorkerHandleRef, MessageEvent)>(
		&self,
		mut new_message_handler: F,
	) -> OldMessageHandler {
		let handle = WorkerHandleRef {
			worker: self.worker.clone(),
			message_handler: Rc::downgrade(&self.message_handler),
		};

		let mut message_handler = RefCell::borrow_mut(&self.message_handler);
		let old_message_handler = message_handler.take();
		let message_handler = message_handler.insert(Closure::classic(move |event| {
			new_message_handler(&handle, MessageEvent::new(event));
		}));

		self.worker.set_onmessage(Some(message_handler));

		OldMessageHandler::new(old_message_handler)
	}

	pub fn set_message_handler_async<
		F1: 'static + FnMut(&WorkerHandleRef, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	>(
		&self,
		mut new_message_handler: F1,
	) -> OldMessageHandler {
		let handle = WorkerHandleRef {
			worker: self.worker.clone(),
			message_handler: Rc::downgrade(&self.message_handler),
		};

		let mut message_handler = RefCell::borrow_mut(&self.message_handler);
		let old_message_handler = message_handler.take();
		let message_handler = message_handler.insert(Closure::future(move |event| {
			new_message_handler(&handle, MessageEvent::new(event))
		}));

		self.worker.set_onmessage(Some(message_handler));

		OldMessageHandler::new(old_message_handler)
	}

	pub fn transfer_messages<M: IntoIterator<Item = I>, I: Into<Message>>(&self, messages: M) {
		WorkerOrContext::Worker(&self.worker).transfer_messages(messages);
	}

	pub fn terminate(self) {
		self.worker.terminate();
	}
}

#[derive(Debug)]
pub struct WorkerHandleRef {
	worker: Worker,
	message_handler: Weak<RefCell<Option<Closure>>>,
}

impl WorkerHandleRef {
	pub(super) fn new(worker: Worker, message_handler: Weak<RefCell<Option<Closure>>>) -> Self {
		Self {
			worker,
			message_handler,
		}
	}

	#[must_use]
	pub const fn raw(&self) -> &Worker {
		&self.worker
	}

	#[must_use]
	pub fn has_message_handler(&self) -> bool {
		Weak::upgrade(&self.message_handler).map_or(false, |message_handler| {
			RefCell::borrow(&message_handler).is_some()
		})
	}

	#[allow(clippy::must_use_candidate)]
	pub fn clear_message_handler(&self) -> OldMessageHandler {
		if let Some(messange_handler) = Weak::upgrade(&self.message_handler) {
			let old_message_handler = messange_handler.take();
			self.worker.set_onmessage(None);

			OldMessageHandler::new(old_message_handler)
		} else {
			OldMessageHandler::new(None)
		}
	}

	pub fn set_message_handler<F: 'static + FnMut(&Self, MessageEvent)>(
		&self,
		mut new_message_handler: F,
	) -> OldMessageHandler {
		if let Some(message_handler) = Weak::upgrade(&self.message_handler) {
			let handle = Self {
				worker: self.worker.clone(),
				message_handler: Weak::clone(&self.message_handler),
			};

			let mut message_handler = RefCell::borrow_mut(&message_handler);
			let old_message_handler = message_handler.take();
			let message_handler = message_handler.insert(Closure::classic(move |event| {
				new_message_handler(&handle, MessageEvent::new(event));
			}));

			self.worker.set_onmessage(Some(message_handler));

			OldMessageHandler::new(old_message_handler)
		} else {
			OldMessageHandler::new(None)
		}
	}

	pub fn set_message_handler_async<
		F1: 'static + FnMut(&Self, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	>(
		&self,
		mut new_message_handler: F1,
	) -> OldMessageHandler {
		if let Some(message_handler) = Weak::upgrade(&self.message_handler) {
			let handle = Self {
				worker: self.worker.clone(),
				message_handler: Weak::clone(&self.message_handler),
			};

			let mut message_handler = RefCell::borrow_mut(&message_handler);
			let old_message_handler = message_handler.take();
			let message_handler = message_handler.insert(Closure::future(move |event| {
				new_message_handler(&handle, MessageEvent::new(event))
			}));

			self.worker.set_onmessage(Some(message_handler));

			OldMessageHandler::new(old_message_handler)
		} else {
			OldMessageHandler::new(None)
		}
	}

	pub fn transfer_messages<M: IntoIterator<Item = I>, I: Into<Message>>(&self, messages: M) {
		WorkerOrContext::Worker(&self.worker).transfer_messages(messages);
	}

	pub fn terminate(self) {
		self.worker.terminate();
	}
}
