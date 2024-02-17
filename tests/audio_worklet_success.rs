#![cfg(test)]
#![cfg(all(target_family = "wasm", feature = "audio-worklet"))]

use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{AudioContext, OfflineAudioContext};
use web_thread::web::audio_worklet::BaseAudioContextExt;

#[wasm_bindgen_test]
async fn register() {
	AudioContext::new()
		.unwrap()
		.register_thread(|| ())
		.await
		.unwrap();
}

#[wasm_bindgen_test]
async fn offline_register() {
	OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
		.unwrap()
		.register_thread(|| ())
		.await
		.unwrap();
}
