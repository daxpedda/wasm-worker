use std::cell::RefCell;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::DedicatedWorkerGlobalScope;

use super::WorkerOrContext;
use crate::{global_with, Global, Message, MessageEvent};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerContext(pub(super) DedicatedWorkerGlobalScope);

impl WorkerContext {
	thread_local! {
		#[allow(clippy::type_complexity)]
		static CLOSURE: RefCell<Option<Closure<dyn FnMut(web_sys::MessageEvent)>>> = RefCell::new(None);
	}

	#[must_use]
	pub fn new() -> Option<Self> {
		global_with(|global| {
			if let Some(Global::DedicatedWorker(global)) = global {
				Some(Self(global.clone()))
			} else {
				None
			}
		})
	}

	#[must_use]
	pub const fn raw(&self) -> &DedicatedWorkerGlobalScope {
		&self.0
	}

	#[allow(clippy::missing_const_for_fn)]
	#[must_use]
	pub fn into_raw(self) -> DedicatedWorkerGlobalScope {
		self.0
	}

	#[must_use]
	pub fn name(&self) -> Option<String> {
		let name = self.0.name();

		if name.is_empty() {
			None
		} else {
			Some(name)
		}
	}

	#[must_use]
	pub fn has_message_handler(&self) -> bool {
		Self::CLOSURE.with(|closure| closure.borrow().is_some())
	}

	pub fn clear_message_handler(&self) {
		Self::CLOSURE.with(|closure| closure.borrow_mut().take());

		self.0.set_onmessage(None);
	}

	pub fn set_message_handler<F: 'static + FnMut(&Self, MessageEvent)>(
		&self,
		mut message_handler: F,
	) {
		Self::CLOSURE.with(|closure| {
			let mut closure = closure.borrow_mut();

			let context = self.clone();
			let closure = closure.insert(Closure::new(move |event| {
				message_handler(&context, MessageEvent::new(event));
			}));

			self.0.set_onmessage(Some(closure.as_ref().unchecked_ref()));
		});
	}

	pub fn transfer_messages<M: IntoIterator<Item = Message>>(&self, messages: M) {
		WorkerOrContext::Context(&self.0).transfer_messages(messages);
	}

	pub fn terminate(self) -> ! {
		__wasm_worker_close();
		unreachable!("continued after terminating");
	}
}

#[wasm_bindgen]
extern "C" {
	fn __wasm_worker_close();
}
