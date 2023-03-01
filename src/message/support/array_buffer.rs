use js_sys::ArrayBuffer;
use once_cell::sync::Lazy;

use super::super::MessageSupportError;

pub(in super::super) fn support() -> Result<(), MessageSupportError> {
	static SUPPORT: Lazy<Result<(), MessageSupportError>> = Lazy::new(|| {
		let buffer = ArrayBuffer::new(1);

		super::test_support(&buffer)
	});

	*SUPPORT
}
