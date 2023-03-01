use once_cell::sync::Lazy;
use web_sys::{VideoFrame, VideoFrameBufferInit, VideoPixelFormat};

use super::super::MessageSupportError;
use crate::global::Global;

pub(in super::super) fn support() -> Result<(), MessageSupportError> {
	static SUPPORT: Lazy<Result<(), MessageSupportError>> = Lazy::new(|| {
		if Global::new().video_frame().is_undefined() {
			return Err(MessageSupportError::Unsupported);
		}

		let frame = VideoFrame::new_with_u8_array_and_video_frame_buffer_init(
			&mut [0; 4],
			&VideoFrameBufferInit::new(1, 1, VideoPixelFormat::Rgba, 0.),
		)
		.unwrap();

		super::test_support(&frame)
	});

	*SUPPORT
}
