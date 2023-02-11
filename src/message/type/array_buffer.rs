use js_sys::ArrayBuffer;
use once_cell::sync::Lazy;

use super::super::SupportError;

pub(in super::super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		let buffer = ArrayBuffer::new(1);

		super::has_support(&buffer)
	});

	*SUPPORT
}
