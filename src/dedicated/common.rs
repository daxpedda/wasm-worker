use std::error::Error;
use std::fmt::{self, Display, Formatter};

use js_sys::Array;
use wasm_bindgen::JsValue;
use web_sys::{DedicatedWorkerGlobalScope, DomException, Worker};

use crate::message::{Message, Messages, RawMessages};

pub(super) enum WorkerOrContext<'this> {
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

	pub(super) fn transfer_messages<M: IntoIterator<Item = I>, I: Into<Message>>(
		self,
		messages: M,
	) -> Result<(), TransferError> {
		let mut messages = messages.into_iter().map(Into::into).map(Message::into_raw);

		let array = 'array: {
			let Some(message_1) = messages.next() else {
				return Ok(())
			};

			let Some(message_2) = messages.next() else {
				let array = Array::of1(&message_1);
				return self
					.post_message_with_transfer(&message_1, &array)
					.map_err(|error| TransferError {
						error: error.into(),
						messages: Messages(RawMessages::Single(message_1)),
					});
			};

			let Some(message_3) = messages.next() else {
				break 'array Array::of2(&message_1, &message_2);
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
			.map_err(|error| TransferError {
				error: error.into(),
				messages: Messages(RawMessages::Array(array)),
			})
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
