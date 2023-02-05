mod builder;

use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::future::Future;

use js_sys::Array;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{DedicatedWorkerGlobalScope, Worker};

pub use self::builder::WorkerBuilder;
use crate::{global_with, Global, Message};

pub fn spawn<F1, F2>(f: F1) -> WorkerHandle
where
	F1: 'static + FnOnce(WorkerContext) -> F2 + Send,
	F2: 'static + Future<Output = Close>,
{
	WorkerBuilder::new().unwrap().spawn(f)
}

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
			message_handler(MessageEvent(event));
		}));

		self.worker
			.set_onmessage(Some(closure.as_ref().unchecked_ref()));
	}

	pub fn transfer_message(&self, message: Message) {
		self.worker
			.post_message_with_transfer(message.as_js_value(), &Array::of1(message.as_js_value()))
			.unwrap_throw();
		drop(message);
	}

	pub fn terminate(self) {
		self.worker.terminate();
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerContext(DedicatedWorkerGlobalScope);

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
				message_handler(&context, MessageEvent(event));
			}));

			self.0.set_onmessage(Some(closure.as_ref().unchecked_ref()));
		});
	}

	pub fn transfer_message(&self, message: Message) {
		self.0
			.post_message_with_transfer(message.as_js_value(), &Array::of1(message.as_js_value()))
			.unwrap_throw();
		drop(message);
	}

	pub fn terminate(self) -> ! {
		__wasm_worker_close();
		unreachable!("continued after terminating");
	}
}

#[derive(Debug)]
pub struct MessageEvent(web_sys::MessageEvent);

impl MessageEvent {
	#[must_use]
	pub fn message(&self) -> Option<Message> {
		Message::from_js_value(self.0.data())
	}

	#[must_use]
	pub const fn raw(&self) -> &web_sys::MessageEvent {
		&self.0
	}

	#[allow(clippy::missing_const_for_fn)]
	#[must_use]
	pub fn into_raw(self) -> web_sys::MessageEvent {
		self.0
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Close {
	Yes,
	No,
}

impl Close {
	const fn to_bool(self) -> bool {
		match self {
			Self::Yes => true,
			Self::No => false,
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleSupportError;

impl Display for ModuleSupportError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "browser doesn't support worker modules")
	}
}

impl Error for ModuleSupportError {}

impl From<ModuleSupportError> for JsValue {
	fn from(value: ModuleSupportError) -> Self {
		value.to_string().into()
	}
}

#[wasm_bindgen]
extern "C" {
	fn __wasm_worker_close();
}
