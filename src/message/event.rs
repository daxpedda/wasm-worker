use std::iter::FusedIterator;
use std::ops::Range;

use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};

use crate::RawMessage;

#[derive(Debug)]
pub struct MessageEvent {
	event: web_sys::MessageEvent,
	messages_taken: bool,
}

impl MessageEvent {
	pub(crate) const fn new(event: web_sys::MessageEvent) -> Self {
		Self {
			event,
			messages_taken: false,
		}
	}

	#[must_use]
	pub fn messages(&self) -> MessageIter {
		if self.messages_taken {
			return MessageIter(Inner::Single(None));
		}

		let data = self.event.data();

		if data.is_array() {
			let array: Array = data.unchecked_into();
			let range = 0..array.length();

			MessageIter(Inner::Array { array, range })
		} else {
			MessageIter(Inner::Single(Some(data)))
		}
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

	fn size_hint(&self) -> (usize, Option<usize>) {
		match &self.0 {
			Inner::Array { range, .. } => range.size_hint(),
			Inner::Single(value) => value.iter().size_hint(),
		}
	}

	fn count(self) -> usize
	where
		Self: Sized,
	{
		self.size_hint().0
	}

	fn last(self) -> Option<Self::Item>
	where
		Self: Sized,
	{
		match self.0 {
			Inner::Array { array, range } => range.last().map(|index| RawMessage(array.get(index))),
			Inner::Single(value) => value.map(RawMessage),
		}
	}

	fn nth(&mut self, n: usize) -> Option<Self::Item> {
		match &mut self.0 {
			Inner::Array { array, range } => range.nth(n).map(|index| RawMessage(array.get(index))),
			Inner::Single(value) => value.take().into_iter().nth(n).map(RawMessage),
		}
	}
}

impl ExactSizeIterator for MessageIter {}

impl FusedIterator for MessageIter {}

impl DoubleEndedIterator for MessageIter {
	fn next_back(&mut self) -> Option<Self::Item> {
		match &mut self.0 {
			Inner::Array { array, range } => {
				range.next_back().map(|index| RawMessage(array.get(index)))
			}
			Inner::Single(value) => value.take().map(RawMessage),
		}
	}

	fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
		match &mut self.0 {
			Inner::Array { array, range } => {
				range.nth_back(n).map(|index| RawMessage(array.get(index)))
			}
			Inner::Single(value) => value.take().into_iter().nth_back(n).map(RawMessage),
		}
	}
}
