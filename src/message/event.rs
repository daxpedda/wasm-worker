use wasm_bindgen::JsCast;

use super::{Messages, RawMessages};

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
	pub fn messages(&self) -> Option<Messages> {
		if self.messages_taken {
			return None;
		}

		let data = self.event.data();

		if data.is_array() {
			Some(Messages(RawMessages::Array(data.unchecked_into())))
		} else if data.is_undefined() {
			Some(Messages(RawMessages::None))
		} else {
			Some(Messages(RawMessages::Single(data)))
		}
	}

	#[must_use]
	pub const fn as_raw(&self) -> &web_sys::MessageEvent {
		&self.event
	}

	#[must_use]
	#[allow(clippy::missing_const_for_fn)]
	pub fn into_raw(self) -> web_sys::MessageEvent {
		self.event
	}
}
