use js_sys::ArrayBuffer;
use once_cell::sync::Lazy;

use super::{util, SupportError};

pub(super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		let buffer = ArrayBuffer::new(1);

		util::has_support(&buffer)
	});

	*SUPPORT
}
