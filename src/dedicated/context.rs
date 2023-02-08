use std::cell::RefCell;
use std::future::Future;

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::DedicatedWorkerGlobalScope;

use super::{Closure, WorkerOrContext};
use crate::{Message, MessageEvent};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerContext(pub(super) DedicatedWorkerGlobalScope);

impl WorkerContext {
	thread_local! {
		#[allow(clippy::type_complexity)]
		static MESSAGE_HANDLER: RefCell<Option<Closure>> = RefCell::new(None);
	}

	#[must_use]
	pub fn new() -> Option<Self> {
		thread_local! {
			static GLOBAL: Option<DedicatedWorkerGlobalScope> = {
				#[wasm_bindgen]
				extern "C" {
					type Global;

					#[wasm_bindgen(method, getter, js_name = DedicatedWorkerGlobalScope)]
					fn worker(this: &Global) -> JsValue;
				}

				let global: Global = js_sys::global().unchecked_into();

				if global.worker().is_undefined() {
					None
				} else {
					Some(global.unchecked_into())
				}
			}
		}

		GLOBAL.with(|global| global.as_ref().cloned()).map(Self)
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
		Self::MESSAGE_HANDLER.with(|message_handler| message_handler.borrow().is_some())
	}

	#[allow(clippy::must_use_candidate)]
	pub fn clear_message_handler(&self) {
		Self::MESSAGE_HANDLER.with(|message_handler| message_handler.borrow_mut().take());
		self.0.set_onmessage(None);
	}

	pub fn set_message_handler<F: 'static + FnMut(&Self, MessageEvent)>(
		&self,
		mut new_message_handler: F,
	) {
		Self::MESSAGE_HANDLER.with(|message_handler| {
			let mut message_handler = message_handler.borrow_mut();

			let context = self.clone();
			let message_handler = message_handler.insert(Closure::classic(move |event| {
				new_message_handler(&context, MessageEvent::new(event));
			}));

			self.0.set_onmessage(Some(message_handler));
		});
	}

	pub fn set_message_handler_async<
		F1: 'static + FnMut(&Self, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	>(
		&self,
		mut new_message_handler: F1,
	) {
		Self::MESSAGE_HANDLER.with(|message_handler| {
			let mut message_handler = message_handler.borrow_mut();

			let context = self.clone();
			let message_handler = message_handler.insert(Closure::future(move |event| {
				new_message_handler(&context, MessageEvent::new(event))
			}));

			self.0.set_onmessage(Some(message_handler));
		});
	}

	pub fn transfer_messages<M: IntoIterator<Item = I>, I: Into<Message>>(&self, messages: M) {
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
