mod image_bitmap;

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

use js_sys::{Array, ArrayBuffer, Object};
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
#[cfg(web_sys_unstable_apis)]
use web_sys::{AudioData, AudioDataInit, AudioSampleFormat, VideoFrame};
use web_sys::{
	ImageBitmap, MessagePort, OffscreenCanvas, ReadableStream, RtcDataChannel, TransformStream,
	Worker, WritableStream,
};

pub use self::image_bitmap::ImageBitmapSupportFuture;

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

impl From<ArrayBuffer> for Message {
	fn from(value: ArrayBuffer) -> Self {
		Self::ArrayBuffer(value)
	}
}

impl TryFrom<Message> for ArrayBuffer {
	type Error = MessageError<Message>;

	fn try_from(value: Message) -> Result<Self, Self::Error> {
		match value {
			Message::ArrayBuffer(value) => Ok(value),
			_ => Err(MessageError(value)),
		}
	}
}

#[cfg(web_sys_unstable_apis)]
impl From<AudioData> for Message {
	fn from(value: AudioData) -> Self {
		Self::AudioData(value)
	}
}

#[cfg(web_sys_unstable_apis)]
impl TryFrom<Message> for AudioData {
	type Error = MessageError<Message>;

	fn try_from(value: Message) -> Result<Self, Self::Error> {
		match value {
			Message::AudioData(value) => Ok(value),
			_ => Err(MessageError(value)),
		}
	}
}

impl From<ImageBitmap> for Message {
	fn from(value: ImageBitmap) -> Self {
		Self::ImageBitmap(value)
	}
}

impl TryFrom<Message> for ImageBitmap {
	type Error = MessageError<Message>;

	fn try_from(value: Message) -> Result<Self, Self::Error> {
		match value {
			Message::ImageBitmap(value) => Ok(value),
			_ => Err(MessageError(value)),
		}
	}
}

impl From<MessagePort> for Message {
	fn from(value: MessagePort) -> Self {
		Self::MessagePort(value)
	}
}

impl TryFrom<Message> for MessagePort {
	type Error = MessageError<Message>;

	fn try_from(value: Message) -> Result<Self, Self::Error> {
		match value {
			Message::MessagePort(value) => Ok(value),
			_ => Err(MessageError(value)),
		}
	}
}

impl From<OffscreenCanvas> for Message {
	fn from(value: OffscreenCanvas) -> Self {
		Self::OffscreenCanvas(value)
	}
}

impl TryFrom<Message> for OffscreenCanvas {
	type Error = MessageError<Message>;

	fn try_from(value: Message) -> Result<Self, Self::Error> {
		match value {
			Message::OffscreenCanvas(value) => Ok(value),
			_ => Err(MessageError(value)),
		}
	}
}

impl From<ReadableStream> for Message {
	fn from(value: ReadableStream) -> Self {
		Self::ReadableStream(value)
	}
}

impl TryFrom<Message> for ReadableStream {
	type Error = MessageError<Message>;

	fn try_from(value: Message) -> Result<Self, Self::Error> {
		match value {
			Message::ReadableStream(value) => Ok(value),
			_ => Err(MessageError(value)),
		}
	}
}

impl From<RtcDataChannel> for Message {
	fn from(value: RtcDataChannel) -> Self {
		Self::RtcDataChannel(value)
	}
}

impl TryFrom<Message> for RtcDataChannel {
	type Error = MessageError<Message>;

	fn try_from(value: Message) -> Result<Self, Self::Error> {
		match value {
			Message::RtcDataChannel(value) => Ok(value),
			_ => Err(MessageError(value)),
		}
	}
}

impl From<TransformStream> for Message {
	fn from(value: TransformStream) -> Self {
		Self::TransformStream(value)
	}
}

impl TryFrom<Message> for TransformStream {
	type Error = MessageError<Message>;

	fn try_from(value: Message) -> Result<Self, Self::Error> {
		match value {
			Message::TransformStream(value) => Ok(value),
			_ => Err(MessageError(value)),
		}
	}
}

#[cfg(web_sys_unstable_apis)]
impl From<VideoFrame> for Message {
	fn from(value: VideoFrame) -> Self {
		Self::VideoFrame(value)
	}
}

#[cfg(web_sys_unstable_apis)]
impl TryFrom<Message> for VideoFrame {
	type Error = MessageError<Message>;

	fn try_from(value: Message) -> Result<Self, Self::Error> {
		match value {
			Message::VideoFrame(value) => Ok(value),
			_ => Err(MessageError(value)),
		}
	}
}

impl From<WritableStream> for Message {
	fn from(value: WritableStream) -> Self {
		Self::WritableStream(value)
	}
}

impl TryFrom<Message> for WritableStream {
	type Error = MessageError<Message>;

	fn try_from(value: Message) -> Result<Self, Self::Error> {
		match value {
			Message::WritableStream(value) => Ok(value),
			_ => Err(MessageError(value)),
		}
	}
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

	#[must_use]
	pub fn has_array_buffer_support() -> bool {
		static SUPPORT: Lazy<bool> = Lazy::new(|| {
			let buffer = ArrayBuffer::new(1);

			let worker = Worker::new("data:,").unwrap_throw();
			worker
				.post_message_with_transfer(&buffer, &Array::of1(&buffer))
				.unwrap_throw();
			worker.terminate();

			buffer.byte_length() == 0
		});

		*SUPPORT
	}

	#[must_use]
	#[cfg(web_sys_unstable_apis)]
	pub fn has_audio_data_support() -> bool {
		static SUPPORT: Lazy<bool> = Lazy::new(|| {
			#[wasm_bindgen]
			extern "C" {
				type AudioDataGlobal;

				#[wasm_bindgen(method, getter, js_name = AudioData)]
				fn audio_data(this: &AudioDataGlobal) -> JsValue;
			}

			let global: AudioDataGlobal = js_sys::global().unchecked_into();

			if global.audio_data().is_undefined() {
				return false;
			}

			let init =
				AudioDataInit::new(&ArrayBuffer::new(1), AudioSampleFormat::U8, 1, 1, 3000., 0.);
			let data = AudioData::new(&init).unwrap_throw();

			let worker = Worker::new("data:,").unwrap_throw();
			worker
				.post_message_with_transfer(&data, &Array::of1(&data))
				.unwrap_throw();
			worker.terminate();

			data.format().is_none()
		});

		*SUPPORT
	}

	#[must_use]
	pub fn has_image_bitmap_support() -> ImageBitmapSupportFuture {
		ImageBitmapSupportFuture::new()
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
			"RtcDataChannel" => Message::RtcDataChannel(object.unchecked_into()),
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
