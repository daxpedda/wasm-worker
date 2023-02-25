//! Tests basic worklet functionality.

mod util;

use util::Flag;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::AudioWorkletExt;
use web_sys::OfflineAudioContext;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`AudioWorkletExt::init_wasm`].
#[wasm_bindgen_test]
async fn test() {
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();

	let flag = Flag::new();
	context
		.init_wasm({
			let flag = flag.clone();
			move |_| flag.signal()
		})
		.unwrap()
		.await
		.unwrap();
}
