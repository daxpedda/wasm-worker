#![allow(clippy::redundant_pub_crate)]
#![allow(
	missing_docs,
	clippy::missing_docs_in_private_items,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc
)]

//! TODO:
//! - Note Chrome nested Worker issue: <https://bugs.chromium.org/p/chromium/issues/detail?id=1408115>.
//! - Document that getting the default worker url will fail if using no-modules
//!   and not starting in a document.
//! - Note possible race condition when sending to newly spawned worker not
//!   receiving messages if receiving message handler wasn't setup yet.

mod dedicated;
mod global;
mod message;
mod worklet;

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

pub use self::dedicated::{
	spawn, spawn_async, Close, MessageEvent, MessageIter, ModuleSupportError, WorkerBuilder,
	WorkerContext, WorkerHandle, WorkerUrl, WorkerUrlFormat,
};
use self::global::{global_with, Global};
pub use self::message::{Message, MessageError, RawMessage};

#[wasm_bindgen]
extern "C" {
	/// JS `try catch` block.
	#[doc(hidden)]
	#[allow(unused_doc_comments)]
	pub fn __wasm_worker_try(fn_: &mut dyn FnMut()) -> JsValue;
}
