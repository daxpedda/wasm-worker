use once_cell::sync::Lazy;
use web_sys::MessageChannel;

use super::super::SupportError;

pub(in super::super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		let channel = MessageChannel::new().unwrap();
		let port = channel.port1();

		super::test_support(&port)
	});

	*SUPPORT
}
