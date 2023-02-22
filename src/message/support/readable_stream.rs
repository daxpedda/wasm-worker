use once_cell::sync::Lazy;
use web_sys::ReadableStream;

use super::super::SupportError;

pub(in super::super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		let stream = ReadableStream::new().unwrap();

		super::test_support(&stream)
	});

	*SUPPORT
}
