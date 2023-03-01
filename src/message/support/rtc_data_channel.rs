use once_cell::sync::Lazy;
use web_sys::RtcPeerConnection;

use super::super::MessageSupportError;

pub(in super::super) fn support() -> Result<(), MessageSupportError> {
	static SUPPORT: Lazy<Result<(), MessageSupportError>> = Lazy::new(|| {
		let connection = RtcPeerConnection::new().unwrap();
		let channel = connection.create_data_channel("");

		super::test_support(&channel)
	});

	*SUPPORT
}
