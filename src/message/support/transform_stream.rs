use once_cell::sync::Lazy;
use web_sys::TransformStream;

use super::super::MessageSupportError;

pub(in super::super) fn support() -> Result<(), MessageSupportError> {
	static SUPPORT: Lazy<Result<(), MessageSupportError>> = Lazy::new(|| {
		let stream = TransformStream::new().unwrap();

		super::test_support(&stream)
	});

	*SUPPORT
}
