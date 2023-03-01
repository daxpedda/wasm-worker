use once_cell::sync::Lazy;
use web_sys::MessageChannel;

use super::super::MessageSupportError;

pub(in super::super) fn support() -> Result<(), MessageSupportError> {
	static SUPPORT: Lazy<Result<(), MessageSupportError>> = Lazy::new(|| {
		let channel = MessageChannel::new().unwrap();
		let port = channel.port1();

		super::test_support(&port)
	});

	*SUPPORT
}
