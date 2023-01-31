#![allow(
	missing_docs,
	clippy::missing_docs_in_private_items,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc
)]

//! TODO:
//! - Note Chrome nested Worker issue: <https://bugs.chromium.org/p/chromium/issues/detail?id=1408115>.

mod dedicated;
mod global;
mod script_url;
mod worklet;

use js_sys::ArrayBuffer;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
#[cfg(web_sys_unstable_apis)]
use web_sys::{AudioData, VideoFrame};
use web_sys::{
	ImageBitmap, MessagePort, OffscreenCanvas, ReadableStream, RtcDataChannel, TransformStream,
	WritableStream,
};

pub use self::dedicated::{name, spawn, terminate, Close, Error, WorkerBuilder, WorkerHandle};
pub use self::global::{global_with, Global};
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

#[wasm_bindgen]
extern "C" {
	/// JS `try catch` block.
	#[doc(hidden)]
	#[allow(unused_doc_comments)]
	pub fn __wasm_worker_try(fn_: &mut dyn FnMut()) -> JsValue;
}
