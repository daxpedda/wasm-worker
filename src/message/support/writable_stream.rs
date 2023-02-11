use once_cell::sync::Lazy;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::WritableStream;

use super::super::SupportError;

pub(in super::super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		let stream = WritableStream::new().unwrap_throw();

		super::test_support(&stream)
	});

	*SUPPORT
}
