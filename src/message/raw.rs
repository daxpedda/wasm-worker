use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

use js_sys::{Array, Object};
use wasm_bindgen::{JsCast, JsValue};

use super::Message;

#[derive(Debug)]
pub struct RawMessage(pub(crate) JsValue);

impl RawMessage {
	#[must_use]
	#[allow(clippy::missing_const_for_fn)]
	pub fn into_raw(self) -> JsValue {
		self.0
	}

	pub fn serialize(self) -> Result<Message, MessageError<Self>> {
		let data = self.0;

		let object = if data.is_object() {
			Object::unchecked_from_js(data)
		} else {
			return Err(MessageError(Self(data)));
		};

		Ok(match String::from(object.constructor().name()).as_str() {
			"ArrayBuffer" => Message::ArrayBuffer(object.unchecked_into()),
			#[cfg(web_sys_unstable_apis)]
			"AudioData" => Message::AudioData(object.unchecked_into()),
			"ImageBitmap" => Message::ImageBitmap(object.unchecked_into()),
			"MessagePort" => Message::MessagePort(object.unchecked_into()),
			"OffscreenCanvas" => Message::OffscreenCanvas(object.unchecked_into()),
			"ReadableStream" => Message::ReadableStream(object.unchecked_into()),
			"RTCDataChannel" => Message::RtcDataChannel(object.unchecked_into()),
			"TransformStream" => Message::TransformStream(object.unchecked_into()),
			#[cfg(web_sys_unstable_apis)]
			"VideoFrame" => Message::VideoFrame(object.unchecked_into()),
			"WritableStream" => Message::WritableStream(object.unchecked_into()),
			_ => return Err(MessageError(Self(object.into()))),
		})
	}

	pub fn serialize_as<T>(self) -> Result<T, MessageError<Self>>
	where
		T: JsCast,
		Message: From<T>,
	{
		self.0.dyn_into().map_err(Self).map_err(MessageError)
	}
}

#[derive(Debug)]
pub enum RawMessages {
	None,
	Single(JsValue),
	Array(Array),
}

#[derive(Debug)]
pub struct MessageError<T>(pub T)
where
	T: Debug;

impl<T> Display for MessageError<T>
where
	T: Debug,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "unexpected message type for: {:?}", self.0)
	}
}

impl<T> Error for MessageError<T> where T: Debug {}
