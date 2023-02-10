use js_sys::{Array, ArrayBuffer};
use once_cell::sync::Lazy;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::Worker;

use super::SupportError;

pub(super) fn has_array_buffer_support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<bool> = Lazy::new(|| {
		let buffer = ArrayBuffer::new(1);

		let worker = Worker::new("data:,").unwrap_throw();
		worker
			.post_message_with_transfer(&buffer, &Array::of1(&buffer))
			.unwrap_throw();
		worker.terminate();

		buffer.byte_length() == 0
	});

	SUPPORT.then_some(()).ok_or(SupportError::Unsupported)
}
