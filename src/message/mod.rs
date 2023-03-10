mod conversion;
mod event;
mod handler;
mod raw;
mod send;
mod support;

use std::fmt::Debug;
use std::iter::FusedIterator;
use std::ops::Range;

use js_sys::{Array, ArrayBuffer};
use wasm_bindgen::JsValue;
#[cfg(web_sys_unstable_apis)]
use web_sys::{AudioData, VideoFrame};
use web_sys::{
	ImageBitmap, MessagePort, OffscreenCanvas, ReadableStream, RtcDataChannel, TransformStream,
	WritableStream,
};

pub use self::event::MessageEvent;
pub(crate) use self::handler::{MessageHandler, SendMessageHandler};
pub use self::raw::{MessageError, RawMessage, RawMessages};
pub(crate) use self::send::SendMessages;
pub use self::send::TransferError;
pub use self::support::{ImageBitmapSupportFuture, MessageSupportError, MessageSupportFuture};

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
	#[must_use]
	pub fn into_raw(self) -> JsValue {
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
}

#[derive(Debug)]
pub struct Messages(pub(crate) RawMessages);

impl Messages {
	#[must_use]
	#[allow(clippy::missing_const_for_fn)]
	pub fn into_raw(self) -> RawMessages {
		self.0
	}
}

impl IntoIterator for Messages {
	type Item = RawMessage;

	type IntoIter = MessageIter;

	fn into_iter(self) -> Self::IntoIter {
		match self.0 {
			RawMessages::None => MessageIter(Inner::Single(None)),
			RawMessages::Single(value) => MessageIter(Inner::Single(Some(value))),
			RawMessages::Array(array) => MessageIter(Inner::Array {
				range: 0..array.length(),
				array,
			}),
		}
	}
}

#[derive(Debug)]
pub struct MessageIter(Inner);

#[derive(Debug)]
enum Inner {
	Single(Option<JsValue>),
	Array { array: Array, range: Range<u32> },
}

impl MessageIter {
	#[must_use]
	pub fn into_raw(self) -> RawMessages {
		match self.0 {
			Inner::Single(None) => RawMessages::None,
			Inner::Single(Some(value)) => RawMessages::Single(value),
			Inner::Array { array, range } => {
				let real_range = 0..array.length();

				if range.is_empty() {
					RawMessages::None
				} else if real_range == range {
					RawMessages::Array(array)
				} else {
					RawMessages::Array(array.slice(range.start, range.end))
				}
			}
		}
	}
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
