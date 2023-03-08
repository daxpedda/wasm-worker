use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

use js_sys::Array;
use wasm_bindgen::JsValue;
use web_sys::{DedicatedWorkerGlobalScope, DomException, MessagePort, Worker};

use super::{Message, Messages, RawMessages};

pub(crate) trait SendMessages {
	fn post_message(&self, message: &JsValue) -> Result<(), JsValue>;

	fn post_message_with_transfer(
		&self,
		message: &JsValue,
		transfer: &JsValue,
	) -> Result<(), JsValue>;

	fn transfer_messages<I: IntoIterator<Item = M>, M: Into<Message>>(
		&self,
		messages: I,
	) -> Result<(), TransferError> {
		match RawMessages::from_messages(messages) {
			RawMessages::None => {
				self.post_message(&JsValue::UNDEFINED)
					.map_err(|error| TransferError {
						error: error.into(),
						messages: Messages(RawMessages::None),
					})
			}
			RawMessages::Single(message) => {
				let transfer = Array::of1(&message);
				self.post_message_with_transfer(&message, &transfer)
					.map_err(|error| TransferError {
						error: error.into(),
						messages: Messages(RawMessages::Single(message)),
					})
			}
			RawMessages::Array(messages) => self
				.post_message_with_transfer(&messages, &messages)
				.map_err(|error| TransferError {
					error: error.into(),
					messages: Messages(RawMessages::Array(messages)),
				}),
		}
	}
}

impl RawMessages {
	pub(crate) fn from_messages<I: IntoIterator<Item = M>, M: Into<Message>>(messages: I) -> Self {
		let mut messages = messages.into_iter().map(Into::into).map(Message::into_raw);

		let Some(message_1) = messages.next() else {
			return Self::None;
		};

		let Some(message_2) = messages.next() else {
			return Self::Single(message_1);
		};

		let Some(message_3) = messages.next() else {
			return Self::Array(Array::of2(&message_1, &message_2));
		};

		let Some(message_4) = messages.next() else {
			return Self::Array(Array::of3(&message_1, &message_2, &message_3));
		};

		if let Some(message_5) = messages.next() {
			let array = Array::of5(&message_1, &message_2, &message_3, &message_4, &message_5);

			for message in messages {
				array.push(&message);
			}

			Self::Array(array)
		} else {
			Self::Array(Array::of4(&message_1, &message_2, &message_3, &message_4))
		}
	}
}

impl SendMessages for Worker {
	fn post_message(&self, message: &JsValue) -> Result<(), JsValue> {
		self.post_message(message)
	}

	fn post_message_with_transfer(
		&self,
		message: &JsValue,
		transfer: &JsValue,
	) -> Result<(), JsValue> {
		self.post_message_with_transfer(message, transfer)
	}
}

impl SendMessages for DedicatedWorkerGlobalScope {
	fn post_message(&self, message: &JsValue) -> Result<(), JsValue> {
		self.post_message(message)
	}

	fn post_message_with_transfer(
		&self,
		message: &JsValue,
		transfer: &JsValue,
	) -> Result<(), JsValue> {
		self.post_message_with_transfer(message, transfer)
	}
}

impl SendMessages for MessagePort {
	fn post_message(&self, message: &JsValue) -> Result<(), JsValue> {
		self.post_message(message)
	}

	fn post_message_with_transfer(
		&self,
		message: &JsValue,
		transfer: &JsValue,
	) -> Result<(), JsValue> {
		self.post_message_with_transferable(message, transfer)
	}
}

#[derive(Debug)]
pub struct TransferError {
	pub error: DomException,
	pub messages: Messages,
}

impl Display for TransferError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "error transferring type: {:?}", self.error)
	}
}

impl Error for TransferError {}
