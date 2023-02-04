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

mod dedicated;
mod global;
mod script_url;
mod worklet;

use js_sys::ArrayBuffer;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
#[cfg(web_sys_unstable_apis)]
use web_sys::{AudioData, VideoFrame};
use web_sys::{
	ImageBitmap, MessagePort, OffscreenCanvas, ReadableStream, RtcDataChannel, TransformStream,
	WritableStream,
};

pub use self::dedicated::{
	spawn, Close, MessageEvent, ModuleSupportError, WorkerBuilder, WorkerContext, WorkerHandle,
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

impl Message {
	fn from_js_value(data: JsValue) -> Option<Self> {
		if ArrayBuffer::is_type_of(&data) {
			return Some(Self::ArrayBuffer(data.unchecked_into()));
		}

		#[cfg(web_sys_unstable_apis)]
		if AudioData::is_type_of(&data) {
			return Some(Self::AudioData(data.unchecked_into()));
		}

		if ImageBitmap::is_type_of(&data) {
			return Some(Self::ImageBitmap(data.unchecked_into()));
		}

		if MessagePort::is_type_of(&data) {
			return Some(Self::MessagePort(data.unchecked_into()));
		}

		if OffscreenCanvas::is_type_of(&data) {
			return Some(Self::OffscreenCanvas(data.unchecked_into()));
		}

		if ReadableStream::is_type_of(&data) {
			return Some(Self::ReadableStream(data.unchecked_into()));
		}

		if RtcDataChannel::is_type_of(&data) {
			return Some(Self::RtcDataChannel(data.unchecked_into()));
		}

		if TransformStream::is_type_of(&data) {
			return Some(Self::TransformStream(data.unchecked_into()));
		}

		#[cfg(web_sys_unstable_apis)]
		if VideoFrame::is_type_of(&data) {
			return Some(Self::VideoFrame(data.unchecked_into()));
		}

		if WritableStream::is_type_of(&data) {
			return Some(Self::WritableStream(data.unchecked_into()));
		}

		None
	}

	fn as_js_value(&self) -> &JsValue {
		match self {
			Self::ArrayBuffer(value) => value,
			#[cfg(web_sys_unstable_apis)]
			Self::AudioData(value) => value,
			Self::ImageBitmap(value) => value,
			Self::MessagePort(value) => value,
			Self::OffscreenCanvas(value) => value,
			Self::ReadableStream(value) => value,
			Self::RtcDataChannel(value) => value,
			Self::TransformStream(value) => value,
			#[cfg(web_sys_unstable_apis)]
			Self::VideoFrame(value) => value,
			Self::WritableStream(value) => value,
		}
	}

	fn has_transfered(&self) -> bool {
		match self {
			Self::ArrayBuffer(array_buffer) => array_buffer.byte_length() == 0,
			#[cfg(web_sys_unstable_apis)]
			Self::AudioData(audio_data) => {
				//web_sys::console::log_1(audio_data);
				todo!()
			}
			Self::ImageBitmap(value) => todo!(),
			Self::MessagePort(value) => todo!(),
			Self::OffscreenCanvas(value) => todo!(),
			Self::ReadableStream(value) => todo!(),
			Self::RtcDataChannel(value) => todo!(),
			Self::TransformStream(value) => todo!(),
			#[cfg(web_sys_unstable_apis)]
			Self::VideoFrame(value) => todo!(),
			Self::WritableStream(value) => todo!(),
		}
	}
}

#[wasm_bindgen]
extern "C" {
	/// JS `try catch` block.
	#[doc(hidden)]
	#[allow(unused_doc_comments)]
	pub fn __wasm_worker_try(fn_: &mut dyn FnMut()) -> JsValue;
}
