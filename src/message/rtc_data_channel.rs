use once_cell::sync::Lazy;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::RtcPeerConnection;

use super::{util, SupportError};

pub(super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		let connection = RtcPeerConnection::new().unwrap_throw();
		let channel = connection.create_data_channel("");

		util::has_support(&channel)
	});

	*SUPPORT
}
