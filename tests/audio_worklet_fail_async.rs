#![cfg(test)]
#![cfg(all(target_family = "wasm", feature = "audio-worklet"))]

use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{AudioContext, OfflineAudioContext};
use web_thread::web::audio_worklet::BaseAudioContextExt;

#[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
use super::test_processor::TestProcessor;

#[wasm_bindgen_test]
#[cfg(any(not(target_feature = "atomics"), unsupported_spawn))]
#[should_panic = "operation not supported on this platform without the atomics target feature and \
                  cross-origin isolation"]
async fn register() {
	AudioContext::new()
		.unwrap()
		.register_thread(|| ())
		.await
		.unwrap();
}

#[wasm_bindgen_test]
#[cfg(any(not(target_feature = "atomics"), unsupported_spawn))]
#[should_panic = "operation not supported on this platform without the atomics target feature and \
                  cross-origin isolation"]
async fn offline_register() {
	OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
		.unwrap()
		.register_thread(|| ())
		.await
		.unwrap();
}

#[wasm_bindgen_test]
#[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
#[should_panic = "`BaseAudioContext` already registered a thread"]
async fn register_twice() {
	let context = AudioContext::new().unwrap();

	context.clone().register_thread(|| ()).await.unwrap();
	context.register_thread(|| ()).await.unwrap();
}

#[wasm_bindgen_test]
#[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
#[should_panic = "`BaseAudioContext` already registered a thread"]
async fn offline_register_twice() {
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();

	context.clone().register_thread(|| ()).await.unwrap();
	context.register_thread(|| ()).await.unwrap();
}

#[wasm_bindgen_test]
#[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
#[should_panic = "name"]
async fn not_registered_node() {
	let context = AudioContext::new().unwrap();
	context.clone().register_thread(|| ()).await.unwrap();
	context
		.audio_worklet_node::<TestProcessor>("test", Box::new(|| None), None)
		.unwrap();
}

#[wasm_bindgen_test]
#[cfg(any(not(target_feature = "atomics"), unsupported_spawn))]
async fn check_failing_spawn() {
	use js_sys::Array;
	use wasm_bindgen_futures::JsFuture;
	use web_sys::{AudioWorkletNode, AudioWorkletNodeOptions, Blob, BlobPropertyBag, Url};

	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();

	let sequence = Array::of1(
		&"registerProcessor('test', class extends AudioWorkletProcessor { constructor() { } \
		  process() { } })"
			.into(),
	);
	let mut property = BlobPropertyBag::new();
	property.type_("text/javascript");
	let blob = Blob::new_with_str_sequence_and_options(&sequence, &property).unwrap();
	let url = Url::create_object_url_with_blob(&blob).unwrap();

	JsFuture::from(context.audio_worklet().unwrap().add_module(&url).unwrap())
		.await
		.unwrap();

	let mut options = AudioWorkletNodeOptions::new();
	options.processor_options(Some(&Array::of1(&wasm_bindgen::memory())));

	AudioWorkletNode::new_with_options(&context, "'test'", &options).unwrap_err();
}
