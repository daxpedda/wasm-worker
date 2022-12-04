#![allow(
	missing_docs,
	clippy::missing_docs_in_private_items,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc
)]

use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;

use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{console, Blob, BlobPropertyBag, DedicatedWorkerGlobalScope, Url, Worker};

static SCRIPT_URL: Lazy<ScriptUrl> = Lazy::new(ScriptUrl::new);

struct ScriptUrl(String);

impl Deref for ScriptUrl {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl ScriptUrl {
	fn new() -> Self {
		let script = format!(
			"importScripts('{}');\n{}",
			wasm_bindgen::script_url(),
			include_str!("script.js")
		);

		let sequence = Array::of1(&JsValue::from(script));
		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property);

		let worker_url = blob
			.and_then(|blob| Url::create_object_url_with_blob(&blob))
			.expect("worker `Url` could not be created");

		Self(worker_url)
	}
}

impl Drop for ScriptUrl {
	fn drop(&mut self) {
		if let Err(error) = Url::revoke_object_url(&self.0) {
			console::warn_1(&format!("worker `Url` could not be deallocated: {error:?}").into());
		}
	}
}

#[derive(Debug)]
pub struct WorkerHandle(Worker);

impl WorkerHandle {
	#[cfg(feature = "raw")]
	#[must_use]
	pub const fn raw(&self) -> &Worker {
		&self.0
	}

	#[cfg(feature = "raw")]
	#[allow(clippy::missing_const_for_fn)]
	#[must_use]
	pub fn into_raw(self) -> Worker {
		self.0
	}

	pub fn terminate(self) {
		self.0.terminate();
	}
}

pub fn spawn<F1, F2>(f: F1) -> WorkerHandle
where
	F1: 'static + FnOnce() -> F2 + Send,
	F2: 'static + Future<Output = Close>,
{
	let work = Task(Box::new(move || Box::pin(async move { f().await })));
	let task = Box::into_raw(Box::new(work));

	let worker = Worker::new(&SCRIPT_URL).expect("`Worker.new()` failed");
	let init = Array::of3(
		&wasm_bindgen::module(),
		&wasm_bindgen::memory(),
		&task.into(),
	);

	worker
		.post_message(&init)
		.expect("`Worker.postMessage()` failed");

	WorkerHandle(worker)
}

#[derive(Clone, Copy, Debug)]
pub enum Close {
	Yes,
	No,
}

impl Close {
	const fn to_bool(self) -> bool {
		match self {
			Self::Yes => true,
			Self::No => false,
		}
	}
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct Task(
	Box<dyn 'static + FnOnce() -> Pin<Box<dyn 'static + Future<Output = Close>>> + Send>,
);

#[doc(hidden)]
#[allow(clippy::future_not_send)]
#[wasm_bindgen]
pub async fn __wasm_worker_entry(task: *mut Task) -> bool {
	js_sys::global()
		.unchecked_into::<DedicatedWorkerGlobalScope>()
		.set_onmessage(None);

	// SAFETY: The argument is an address that has to be a valid pointer to a
	// `Task`.
	let Task(work) = *unsafe { Box::from_raw(task) };

	let close = work().await;

	close.to_bool()
}

#[wasm_bindgen]
extern "C" {
	fn __wasm_worker_close();
}
