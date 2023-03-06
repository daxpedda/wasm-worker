use once_cell::sync::OnceCell;
use web_sys::OffscreenCanvas;

use super::super::MessageSupportError;
use crate::global::{Global, GlobalContext};

pub(in super::super) fn support() -> Result<bool, MessageSupportError> {
	static SUPPORT: OnceCell<bool> = OnceCell::new();

	SUPPORT
		.get_or_try_init(|| {
			GlobalContext::with(|global| match global {
				GlobalContext::Window(_) => Ok(()),
				GlobalContext::Worker(_) => {
					if Global::has_worker() {
						Ok(())
					} else {
						Err(MessageSupportError)
					}
				}
				GlobalContext::Worklet => Err(MessageSupportError),
			})?;

			if Global::with(Global::offscreen_canvas).is_undefined() {
				return Ok(false);
			}

			let canvas = OffscreenCanvas::new(1, 0).unwrap();

			Ok(super::test_support(&canvas))
		})
		.copied()
}
