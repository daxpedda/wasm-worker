pub(super) mod array_buffer;
#[cfg(web_sys_unstable_apis)]
pub(super) mod audio_data;
pub(super) mod image_bitmap;
pub(super) mod message_port;
pub(super) mod offscreen_canvas;
pub(super) mod readable_stream;
pub(super) mod rtc_data_channel;
pub(super) mod transform_stream;
#[cfg(web_sys_unstable_apis)]
pub(super) mod video_frame;
pub(super) mod writable_stream;

use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{DomException, Worker};

use super::SupportError;

fn has_support(data: &JsValue) -> Result<(), SupportError> {
	let worker = Worker::new("data:,").unwrap_throw();
	let result = worker.post_message_with_transfer(data, &Array::of1(data));
	worker.terminate();

	if let Err(error) = result {
		let error: DomException = error.unchecked_into();

		if error.code() == DomException::DATA_CLONE_ERR {
			Err(SupportError::Unsupported)
		} else {
			Err(SupportError::Undetermined)
		}
	} else {
		Ok(())
	}
}
