use js_sys::ArrayBuffer;
use once_cell::sync::Lazy;
use web_sys::{AudioData, AudioDataInit, AudioSampleFormat};

use super::super::MessageSupportError;
use crate::global::Global;

pub(in super::super) fn support() -> Result<(), MessageSupportError> {
	static SUPPORT: Lazy<Result<(), MessageSupportError>> = Lazy::new(|| {
		let global = Global::new();

		if global.audio_data().is_undefined() {
			return Err(MessageSupportError::Unsupported);
		}

		let init = AudioDataInit::new(&ArrayBuffer::new(1), AudioSampleFormat::U8, 1, 1, 3000., 0.);
		let data = AudioData::new(&init).unwrap();

		super::test_support(&data)
	});

	*SUPPORT
}
