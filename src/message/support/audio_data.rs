use js_sys::ArrayBuffer;
use once_cell::sync::Lazy;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::{AudioData, AudioDataInit, AudioSampleFormat};

use super::super::SupportError;
use crate::global::Global;

pub(in super::super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		let global = Global::new();

		if global.audio_data().is_undefined() {
			return Err(SupportError::Unsupported);
		}

		let init = AudioDataInit::new(&ArrayBuffer::new(1), AudioSampleFormat::U8, 1, 1, 3000., 0.);
		let data = AudioData::new(&init).unwrap_throw();

		super::test_support(&data)
	});

	*SUPPORT
}
