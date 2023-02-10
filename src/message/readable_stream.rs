use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::{ReadableStream, Worker};

use super::SupportError;

pub(super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<bool> = Lazy::new(|| {
		let stream = ReadableStream::new().unwrap_throw();

		let worker = Worker::new("data:,").unwrap_throw();
		worker
			.post_message_with_transfer(&stream, &Array::of1(&stream))
			.unwrap_throw();
		worker.terminate();

		stream.locked()
	});

	SUPPORT.then_some(()).ok_or(SupportError::Unsupported)
}
