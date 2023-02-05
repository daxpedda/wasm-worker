#![allow(clippy::redundant_pub_crate)]
#![allow(
	missing_docs,
	clippy::missing_docs_in_private_items,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc
)]

//! TODO:
//! - Note Chrome nested Worker issue: <https://bugs.chromium.org/p/chromium/issues/detail?id=1408115>.
//! - Document that getting the default script url will fail if using no-modules
//!   and not starting in a document.
//! - Note possible race condition when sending to newly spawned worker not
//!   receiving messages if receiving message handler wasn't setup yet.

mod dedicated;
mod global;
mod script_url;
mod worklet;

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use js_sys::{Array, ArrayBuffer, Uint8Array};
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
#[cfg(web_sys_unstable_apis)]
use web_sys::{AudioData, VideoFrame};
use web_sys::{
	ImageBitmap, MessagePort, OffscreenCanvas, ReadableStream, RtcDataChannel, TransformStream,
	Worker, WritableStream,
};

pub use self::dedicated::{
	spawn, Close, MessageEvent, MessageIter, ModuleSupportError, RawMessage, WorkerBuilder,
	WorkerContext, WorkerHandle,
};
use self::global::{global_with, Global};
pub use self::script_url::{default_script_url, ScriptFormat, ScriptUrl};

#[derive(Debug)]
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

impl Message {
	fn from_js_value(data: JsValue) -> Result<Self, MessageError> {
		if data.is_instance_of::<ArrayBuffer>() {
			return Ok(Self::ArrayBuffer(data.unchecked_into()));
		}

		#[cfg(web_sys_unstable_apis)]
		if data.is_instance_of::<AudioData>() {
			return Ok(Self::AudioData(data.unchecked_into()));
		}

		if data.is_instance_of::<ImageBitmap>() {
			return Ok(Self::ImageBitmap(data.unchecked_into()));
		}

		if data.is_instance_of::<MessagePort>() {
			return Ok(Self::MessagePort(data.unchecked_into()));
		}

		if data.is_instance_of::<OffscreenCanvas>() {
			return Ok(Self::OffscreenCanvas(data.unchecked_into()));
		}

		if data.is_instance_of::<ReadableStream>() {
			return Ok(Self::ReadableStream(data.unchecked_into()));
		}

		if data.is_instance_of::<RtcDataChannel>() {
			return Ok(Self::RtcDataChannel(data.unchecked_into()));
		}

		if data.is_instance_of::<TransformStream>() {
			return Ok(Self::TransformStream(data.unchecked_into()));
		}

		#[cfg(web_sys_unstable_apis)]
		if data.is_instance_of::<VideoFrame>() {
			return Ok(Self::VideoFrame(data.unchecked_into()));
		}

		if data.is_instance_of::<WritableStream>() {
			return Ok(Self::WritableStream(data.unchecked_into()));
		}

		Err(MessageError(data))
	}

	fn into_js_value(self) -> JsValue {
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
			let array = Uint8Array::new(&buffer);
			array.copy_from(&[0]);

			let worker = Worker::new("data:,").unwrap_throw();
			worker
				.post_message_with_transfer(&buffer, &Array::of1(&buffer))
				.unwrap_throw();
			worker.terminate();

			buffer.byte_length() == 0
		});

		*SUPPORT
	}
}

#[derive(Debug)]
pub struct MessageError(JsValue);

impl Display for MessageError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "unexpected message: {:?}", self.0)
	}
}

impl Error for MessageError {}

impl From<MessageError> for JsValue {
	fn from(value: MessageError) -> Self {
		value.to_string().into()
	}
}

impl MessageError {
	#[must_use]
	#[allow(clippy::missing_const_for_fn)]
	pub fn into_raw(self) -> JsValue {
		self.0
	}
}

#[wasm_bindgen]
extern "C" {
	/// JS `try catch` block.
	#[doc(hidden)]
	#[allow(unused_doc_comments)]
	pub fn __wasm_worker_try(fn_: &mut dyn FnMut()) -> JsValue;
}
