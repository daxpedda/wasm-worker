use once_cell::sync::OnceCell;
use web_sys::{VideoFrame, VideoFrameBufferInit, VideoPixelFormat};

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

			if Global::with(Global::video_frame).is_undefined() {
				return Ok(false);
			}

			let frame = VideoFrame::new_with_u8_array_and_video_frame_buffer_init(
				&mut [0; 4],
				&VideoFrameBufferInit::new(1, 1, VideoPixelFormat::Rgba, 0.),
			)
			.unwrap();

			Ok(super::test_support(&frame))
		})
		.copied()
}
