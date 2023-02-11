use once_cell::sync::Lazy;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::TransformStream;

use super::{util, SupportError};

pub(super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		let stream = TransformStream::new().unwrap_throw();

		util::has_support(&stream)
	});

	*SUPPORT
}
