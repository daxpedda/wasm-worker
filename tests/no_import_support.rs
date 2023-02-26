//! Tests for failures in various functions when imports are not supported in
//! worklets but the ES module target was used when compiling the JS shim.
//!
//! Not tested by default!

use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::common::ShimFormat;
use wasm_worker::worklet::{WorkletModule, WorkletModuleError};
use wasm_worker::AudioWorkletExt;
use web_sys::OfflineAudioContext;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`AudioWorkletExt::init_wasm`].
#[wasm_bindgen_test]
async fn audio_init() {
	assert!(matches!(
		wasm_bindgen::shim_format(),
		Some(wasm_bindgen::ShimFormat::EsModule)
	));

	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	let result = context.init_wasm(|_| ()).unwrap().await;

	assert!(matches!(result.unwrap_err(), WorkletModuleError::Support));
}

/// [`WorkletModule::has_import_support()`].
#[wasm_bindgen_test]
async fn check() {
	assert!(matches!(
		wasm_bindgen::shim_format(),
		Some(wasm_bindgen::ShimFormat::EsModule)
	));

	assert!(!WorkletModule::has_import_support().await);
}

/// [`WorkletModule::new())`].
#[wasm_bindgen_test]
async fn url() {
	assert!(matches!(
		wasm_bindgen::shim_format(),
		Some(wasm_bindgen::ShimFormat::EsModule)
	));

	let result = WorkletModule::new(&wasm_bindgen::shim_url().unwrap(), ShimFormat::EsModule).await;
	assert!(matches!(result.unwrap_err(), WorkletModuleError::Support));
}
