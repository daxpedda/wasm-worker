mod builder;
mod common;
mod context;
mod handle;

use std::future::Future;
use std::ops::Range;

use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};

pub use self::builder::{ModuleSupportError, WorkerBuilder};
use self::common::WorkerOrContext;
pub use self::context::WorkerContext;
pub use self::handle::WorkerHandle;
use crate::RawMessage;

pub fn spawn<F1, F2>(f: F1) -> WorkerHandle
where
	F1: 'static + FnOnce(WorkerContext) -> F2 + Send,
	F2: 'static + Future<Output = Close>,
{
	WorkerBuilder::new().unwrap().spawn(f)
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
