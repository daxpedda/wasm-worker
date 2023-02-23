use once_cell::sync::Lazy;
use web_sys::OffscreenCanvas;

use super::super::SupportError;
use crate::global::Global;

pub(in super::super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		if Global::new().offscreen_canvas().is_undefined() {
			return Err(SupportError::Unsupported);
		}

		let canvas = OffscreenCanvas::new(1, 0).unwrap();

		super::test_support(&canvas)
	});

	*SUPPORT
}
