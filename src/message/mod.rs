mod conversion;
mod event;
mod has_support;
mod r#type;

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

use js_sys::{ArrayBuffer, Object};
use wasm_bindgen::{JsCast, JsValue};
#[cfg(web_sys_unstable_apis)]
use web_sys::{AudioData, VideoFrame};
use web_sys::{
	ImageBitmap, MessagePort, OffscreenCanvas, ReadableStream, RtcDataChannel, TransformStream,
	WritableStream,
};

pub use self::event::{MessageEvent, MessageIter};
pub use self::has_support::HasSupportFuture;
pub use self::image_bitmap::ImageBitmapSupportFuture;
use self::r#type::{
	array_buffer, image_bitmap, message_port, offscreen_canvas, readable_stream, rtc_data_channel,
	transform_stream, writable_stream,
};
#[cfg(web_sys_unstable_apis)]
use self::r#type::{audio_data, video_frame};

#[derive(Debug, Eq, PartialEq)]
pub enum Message {
	ArrayBuffer(ArrayBuffer),
	#[cfg(web_sys_unstable_apis)]
	AudioData(AudioData),
	ImageBitmap(ImageBitmap),
	MessagePort(MessagePort),
	OffscreenCanvas(OffscreenCanvas),
	ReadableStream(ReadableStream),
	RtcDataChannel(RtcDataChannel),
	TransformStream(TransformStream),
	#[cfg(web_sys_unstable_apis)]
	VideoFrame(VideoFrame),
	WritableStream(WritableStream),
}

impl Message {
	pub(crate) fn into_js_value(self) -> JsValue {
		match self {
			Self::ArrayBuffer(value) => value.into(),
			#[cfg(web_sys_unstable_apis)]
			Self::AudioData(value) => value.into(),
			Self::ImageBitmap(value) => value.into(),
			Self::MessagePort(value) => value.into(),
			Self::OffscreenCanvas(value) => value.into(),
			Self::ReadableStream(value) => value.into(),
			Self::RtcDataChannel(value) => value.into(),
			Self::TransformStream(value) => value.into(),
			#[cfg(web_sys_unstable_apis)]
			Self::VideoFrame(value) => value.into(),
			Self::WritableStream(value) => value.into(),
		}
	}

	pub fn has_support(&self) -> HasSupportFuture {
		HasSupportFuture::new(self)
	}

	pub fn has_array_buffer_support() -> Result<(), SupportError> {
		array_buffer::support()
	}

	#[cfg(web_sys_unstable_apis)]
	pub fn has_audio_data_support() -> Result<(), SupportError> {
		audio_data::support()
	}

	pub fn has_image_bitmap_support() -> ImageBitmapSupportFuture {
		ImageBitmapSupportFuture::new()
	}

	pub fn has_message_port_support() -> Result<(), SupportError> {
		message_port::support()
	}

	pub fn has_offscreen_canvas_support() -> Result<(), SupportError> {
		offscreen_canvas::support()
	}

	pub fn has_readable_stream_support() -> Result<(), SupportError> {
		readable_stream::support()
	}

	pub fn has_rtc_data_channel_support() -> Result<(), SupportError> {
		rtc_data_channel::support()
	}

	pub fn has_transform_stream_support() -> Result<(), SupportError> {
		transform_stream::support()
	}

	#[cfg(web_sys_unstable_apis)]
	pub fn has_video_frame_support() -> Result<(), SupportError> {
		video_frame::support()
	}

	pub fn has_writable_stream_support() -> Result<(), SupportError> {
		writable_stream::support()
	}
}

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
		if self.0.is_instance_of::<T>() {
			Ok(self.0.unchecked_into::<T>())
		} else {
			Err(MessageError(self))
		}
	}
}

#[derive(Debug)]
pub struct MessageError<T: Debug>(pub T);

impl<T: Debug> Display for MessageError<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "unexpected message: {:?}", self.0)
	}
}

impl<T: Debug> Error for MessageError<T> {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportError {
	Unsupported,
	Undetermined,
}

impl Display for SupportError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Unsupported => write!(f, "type is not transferable"),
			Self::Undetermined => write!(f, "type transfer support couldn't be determined"),
		}
	}
}

impl Error for SupportError {}
