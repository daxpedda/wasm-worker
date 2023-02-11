use js_sys::ArrayBuffer;
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{AudioData, AudioDataInit, AudioSampleFormat};

use super::super::SupportError;

pub(in super::super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		#[wasm_bindgen]
		extern "C" {
			type AudioDataGlobal;

			#[wasm_bindgen(method, getter, js_name = AudioData)]
			fn audio_data(this: &AudioDataGlobal) -> JsValue;
		}

		let global: AudioDataGlobal = js_sys::global().unchecked_into();

		if global.audio_data().is_undefined() {
			return Err(SupportError::Unsupported);
		}

		let init = AudioDataInit::new(&ArrayBuffer::new(1), AudioSampleFormat::U8, 1, 1, 3000., 0.);
		let data = AudioData::new(&init).unwrap_throw();

		super::has_support(&data)
	});

	*SUPPORT
}