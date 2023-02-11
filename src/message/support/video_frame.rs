use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{VideoFrame, VideoFrameBufferInit, VideoPixelFormat};

use super::super::SupportError;

pub(in super::super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		#[wasm_bindgen]
		extern "C" {
			#[allow(non_camel_case_types)]
			type __wasm_worker_VideoFrameGlobal;

			#[wasm_bindgen(method, getter, js_name = VideoFrame)]
			fn video_frame(this: &__wasm_worker_VideoFrameGlobal) -> JsValue;
		}

		let global: __wasm_worker_VideoFrameGlobal = js_sys::global().unchecked_into();

		if global.video_frame().is_undefined() {
			return Err(SupportError::Unsupported);
		}

		let frame = VideoFrame::new_with_u8_array_and_video_frame_buffer_init(
			&mut [0; 4],
			&VideoFrameBufferInit::new(1, 1, VideoPixelFormat::Rgba, 0.),
		)
		.unwrap_throw();

		super::test_support(&frame)
	});

	*SUPPORT
}
