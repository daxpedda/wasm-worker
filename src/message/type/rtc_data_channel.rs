use once_cell::sync::Lazy;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::RtcPeerConnection;

use super::super::SupportError;

pub(in super::super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		let connection = RtcPeerConnection::new().unwrap_throw();
		let channel = connection.create_data_channel("");

		super::has_support(&channel)
	});

	*SUPPORT
}
