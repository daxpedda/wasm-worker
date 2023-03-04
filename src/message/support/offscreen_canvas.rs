use once_cell::sync::OnceCell;
use web_sys::OffscreenCanvas;

use super::super::MessageSupportError;
use crate::global::{Global, WindowOrWorker};

pub(in super::super) fn support() -> Result<bool, MessageSupportError> {
	static SUPPORT: OnceCell<bool> = OnceCell::new();

	SUPPORT
		.get_or_try_init(|| {
			WindowOrWorker::with(|global| {
				if Global::new().offscreen_canvas().is_undefined() {
					return Ok(false);
				}

				if let WindowOrWorker::Worker(_) = global {
					if Global::new().worker().is_undefined() {
						return Err(MessageSupportError);
					}
				}

				let canvas = OffscreenCanvas::new(1, 0).unwrap();

				Ok(super::test_support(&canvas))
			})
			.unwrap_or(Err(MessageSupportError))
		})
		.copied()
}
