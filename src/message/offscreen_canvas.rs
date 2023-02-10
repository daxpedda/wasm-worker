use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::{OffscreenCanvas, Worker};

use super::SupportError;

pub(super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<bool> = Lazy::new(|| {
		let canvas = OffscreenCanvas::new(1, 0).unwrap_throw();

		let worker = Worker::new("data:,").unwrap_throw();
		worker
			.post_message_with_transfer(&canvas, &Array::of1(&canvas))
			.unwrap_throw();
		worker.terminate();

		canvas.width() == 0
	});

	SUPPORT.then_some(()).ok_or(SupportError::Unsupported)
}
