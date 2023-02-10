use js_sys::{Array, ArrayBuffer};
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{AudioData, AudioDataInit, AudioSampleFormat, Worker};

use super::SupportError;

pub(super) fn has_audio_data_support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<bool> = Lazy::new(|| {
		#[wasm_bindgen]
		extern "C" {
			type AudioDataGlobal;

			#[wasm_bindgen(method, getter, js_name = AudioData)]
			fn audio_data(this: &AudioDataGlobal) -> JsValue;
		}

		let global: AudioDataGlobal = js_sys::global().unchecked_into();

		if global.audio_data().is_undefined() {
			return false;
		}

		let init = AudioDataInit::new(&ArrayBuffer::new(1), AudioSampleFormat::U8, 1, 1, 3000., 0.);
		let data = AudioData::new(&init).unwrap_throw();

		let worker = Worker::new("data:,").unwrap_throw();
		worker
			.post_message_with_transfer(&data, &Array::of1(&data))
			.unwrap_throw();
		worker.terminate();

		data.format().is_none()
	});

	SUPPORT.then_some(()).ok_or(SupportError::Unsupported)
}
