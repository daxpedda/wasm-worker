use once_cell::sync::Lazy;
use web_sys::TransformStream;

use super::super::SupportError;

pub(in super::super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		let stream = TransformStream::new().unwrap();

		super::test_support(&stream)
	});

	*SUPPORT
}
