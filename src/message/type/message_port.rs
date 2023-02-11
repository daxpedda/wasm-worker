use once_cell::sync::Lazy;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::MessageChannel;

use super::super::SupportError;

pub(in super::super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		let channel = MessageChannel::new().unwrap_throw();
		let port = channel.port1();

		super::has_support(&port)
	});

	*SUPPORT
}
