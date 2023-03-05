use once_cell::sync::OnceCell;
use web_sys::{VideoFrame, VideoFrameBufferInit, VideoPixelFormat};

use super::super::MessageSupportError;
use crate::global::{Global, WindowOrWorker};

pub(in super::super) fn support() -> Result<bool, MessageSupportError> {
	static SUPPORT: OnceCell<bool> = OnceCell::new();

	SUPPORT
		.get_or_try_init(|| {
			WindowOrWorker::with(|global| {
				if Global::with(Global::video_frame).is_undefined() {
					return Ok(false);
				}

				if let WindowOrWorker::Worker(_) = global {
					if !Global::has_worker() {
						return Err(MessageSupportError);
					}
				}

				let frame = VideoFrame::new_with_u8_array_and_video_frame_buffer_init(
					&mut [0; 4],
					&VideoFrameBufferInit::new(1, 1, VideoPixelFormat::Rgba, 0.),
				)
				.unwrap();

				Ok(super::test_support(&frame))
			})
			.unwrap_or(Err(MessageSupportError))
		})
		.copied()
}
