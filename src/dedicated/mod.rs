mod builder;
mod handle;

use std::cell::RefCell;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::ops::Range;

use js_sys::Array;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{DedicatedWorkerGlobalScope, Worker};

pub use self::builder::WorkerBuilder;
pub use self::handle::WorkerHandle;
use crate::{global_with, Global, Message, RawMessage};

pub fn spawn<F1, F2>(f: F1) -> WorkerHandle
where
	F1: 'static + FnOnce(WorkerContext) -> F2 + Send,
	F2: 'static + Future<Output = Close>,
{
	WorkerBuilder::new().unwrap().spawn(f)
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

enum WorkerOrContext<'this> {
	Worker(&'this Worker),
	Context(&'this DedicatedWorkerGlobalScope),
}

impl WorkerOrContext<'_> {
	fn post_message_with_transfer(
		self,
		message: &JsValue,
		transfer: &JsValue,
	) -> Result<(), JsValue> {
		match self {
			WorkerOrContext::Worker(worker) => worker.post_message_with_transfer(message, transfer),
			WorkerOrContext::Context(context) => {
				context.post_message_with_transfer(message, transfer)
			}
		}
	}

	fn transfer_messages<M: IntoIterator<Item = Message>>(self, messages: M) {
		let mut messages = messages.into_iter().map(Message::into_js_value);

		let array = 'array: {
			let Some(message_1) = messages.next() else {
				return
			};

			let Some(message_2) = messages.next() else {
				return self
					.post_message_with_transfer(&message_1, &Array::of1(&message_1))
					.unwrap_throw();
			};

			let Some(message_3) = messages.next() else {
				break 'array  Array::of2(&message_1, &message_2);
			};

			let Some(message_4) = messages.next() else {
				break 'array Array::of3(&message_1, &message_2, &message_3);
			};

			if let Some(message_5) = messages.next() {
				let array = Array::of5(&message_1, &message_2, &message_3, &message_4, &message_5);

				for message in messages {
					array.push(&message);
				}

				array
			} else {
				Array::of4(&message_1, &message_2, &message_3, &message_4)
			}
		};

		self.post_message_with_transfer(&array, &array)
			.unwrap_throw();
	}
}

#[derive(Debug)]
pub struct MessageEvent {
	event: web_sys::MessageEvent,
	message_taken: bool,
}

impl MessageEvent {
	const fn new(event: web_sys::MessageEvent) -> Self {
		Self {
			event,
			message_taken: false,
		}
	}

	#[must_use]
	pub fn messages(&self) -> Option<MessageIter> {
		if self.message_taken {
			return None;
		}

		let data = self.event.data();

		Some(if data.is_array() {
			let array: Array = data.unchecked_into();
			let range = 0..array.length();

			MessageIter(Inner::Array { array, range })
		} else {
			MessageIter(Inner::Single(Some(data)))
		})
	}

	#[must_use]
	pub const fn raw(&self) -> &web_sys::MessageEvent {
		&self.event
	}

	#[allow(clippy::missing_const_for_fn)]
	#[must_use]
	pub fn into_raw(self) -> web_sys::MessageEvent {
		self.event
	}
}

#[derive(Debug)]
pub struct MessageIter(Inner);

#[derive(Debug)]
enum Inner {
	Single(Option<JsValue>),
	Array { array: Array, range: Range<u32> },
}

impl Iterator for MessageIter {
	type Item = RawMessage;

	fn next(&mut self) -> Option<Self::Item> {
		match &mut self.0 {
			Inner::Array { array, range } => {
				let index = range.next()?;
				Some(RawMessage(array.get(index)))
			}
			Inner::Single(value) => value.take().map(RawMessage),
		}
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
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
