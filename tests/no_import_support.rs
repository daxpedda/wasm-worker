//! Tests for failures in various functions when imports are not supported in
//! worklets but the ES module target was used when compiling the JS shim.
//!
//! Not tested by default!

use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::common::ShimFormat;
use wasm_worker::worklet::{WorkletUrl, WorkletUrlError};
use wasm_worker::WorkletExt;
use web_sys::OfflineAudioContext;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`WorkletExt::add_wasm`].
#[wasm_bindgen_test]
async fn init() {
	assert!(matches!(
		wasm_bindgen::shim_format(),
		Some(wasm_bindgen::ShimFormat::EsModule)
	));

	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	let result = context.add_wasm(|_| ()).await;

	assert!(matches!(result.unwrap_err(), WorkletUrlError::Support));
}

/// [`WorkletUrl::has_import_support()`].
#[wasm_bindgen_test]
async fn check() {
	assert!(matches!(
		wasm_bindgen::shim_format(),
		Some(wasm_bindgen::ShimFormat::EsModule)
	));

	assert!(!WorkletUrl::has_import_support().await);
}

/// [`WorkletUrl::new())`].
#[wasm_bindgen_test]
async fn url() {
	assert!(matches!(
		wasm_bindgen::shim_format(),
		Some(wasm_bindgen::ShimFormat::EsModule)
	));

	let result = WorkletUrl::new(&wasm_bindgen::shim_url().unwrap(), ShimFormat::EsModule).await;
	assert!(matches!(result.unwrap_err(), WorkletUrlError::Support));
}
