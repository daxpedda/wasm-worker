use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{DomException, Worker};

use super::SupportError;

pub(super) fn has_support(data: &JsValue) -> Result<(), SupportError> {
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
