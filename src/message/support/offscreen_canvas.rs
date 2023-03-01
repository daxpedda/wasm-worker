use once_cell::sync::Lazy;
use web_sys::OffscreenCanvas;

use super::super::MessageSupportError;
use crate::global::Global;

pub(in super::super) fn support() -> Result<(), MessageSupportError> {
	static SUPPORT: Lazy<Result<(), MessageSupportError>> = Lazy::new(|| {
		if Global::new().offscreen_canvas().is_undefined() {
			return Err(MessageSupportError::Unsupported);
		}

		let canvas = OffscreenCanvas::new(1, 0).unwrap();

		super::test_support(&canvas)
	});

	*SUPPORT
}
