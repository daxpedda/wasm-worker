use js_sys::ArrayBuffer;
#[cfg(web_sys_unstable_apis)]
use web_sys::{AudioData, VideoFrame, WebTransportReceiveStream, WebTransportSendStream};
use web_sys::{
	ImageBitmap, MessagePort, OffscreenCanvas, ReadableStream, RtcDataChannel, TransformStream,
	WritableStream,
};

use super::{Message, MessageError};

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

#[cfg(web_sys_unstable_apis)]
impl From<WebTransportReceiveStream> for Message {
	fn from(value: WebTransportReceiveStream) -> Self {
		Self::WebTransportReceiveStream(value)
	}
}

#[cfg(web_sys_unstable_apis)]
impl TryFrom<Message> for WebTransportReceiveStream {
	type Error = MessageError<Message>;

	fn try_from(value: Message) -> Result<Self, Self::Error> {
		match value {
			Message::WebTransportReceiveStream(value) => Ok(value),
			_ => Err(MessageError(value)),
		}
	}
}

#[cfg(web_sys_unstable_apis)]
impl From<WebTransportSendStream> for Message {
	fn from(value: WebTransportSendStream) -> Self {
		Self::WebTransportSendStream(value)
	}
}

#[cfg(web_sys_unstable_apis)]
impl TryFrom<Message> for WebTransportSendStream {
	type Error = MessageError<Message>;

	fn try_from(value: Message) -> Result<Self, Self::Error> {
		match value {
			Message::WebTransportSendStream(value) => Ok(value),
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
