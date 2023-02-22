use once_cell::sync::Lazy;
use web_sys::RtcPeerConnection;

use super::super::SupportError;

pub(in super::super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		let connection = RtcPeerConnection::new().unwrap();
		let channel = connection.create_data_channel("");

		super::test_support(&channel)
	});

	*SUPPORT
}
