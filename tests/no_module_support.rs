//! Tests for failures in various functions when ES modules are not supported
//! but the ES module target was used when compiling the JS shim.
//!
//! Not tested by default!

use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::common::ShimFormat;
use wasm_worker::dedicated::{ModuleSupportError, WorkerUrl};
use wasm_worker::WorkerBuilder;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`wasm_worker::spawn()`].
#[wasm_bindgen_test]
#[should_panic(expected = "ModuleSupportError")]
fn spawn() {
	assert!(matches!(
		wasm_bindgen::shim_format(),
		Some(wasm_bindgen::ShimFormat::EsModule)
	));

	wasm_worker::spawn(|_| unreachable!());
}

/// [`wasm_worker::spawn_async()`].
#[wasm_bindgen_test]
#[should_panic(expected = "ModuleSupportError")]
fn spawn_async() {
	assert!(matches!(
		wasm_bindgen::shim_format(),
		Some(wasm_bindgen::ShimFormat::EsModule)
	));

	wasm_worker::spawn_async(|_| async { unreachable!() });
}

/// [`WorkerBuilder::has_module_support()`].
#[wasm_bindgen_test]
fn check() {
	assert!(matches!(
		wasm_bindgen::shim_format(),
		Some(wasm_bindgen::ShimFormat::EsModule)
	));

	assert!(!WorkerBuilder::has_module_support());
}

/// [`WorkerBuilder::new()`].
#[wasm_bindgen_test]
fn builder() {
	assert!(matches!(
		wasm_bindgen::shim_format(),
		Some(wasm_bindgen::ShimFormat::EsModule)
	));

	assert_eq!(WorkerBuilder::new().unwrap_err(), ModuleSupportError);
}

/// [`WorkerBuilder::new_with_url()`].
#[wasm_bindgen_test]
fn builder_url() {
	assert!(matches!(
		wasm_bindgen::shim_format(),
		Some(wasm_bindgen::ShimFormat::EsModule)
	));

	let url = WorkerUrl::new(&wasm_bindgen::shim_url().unwrap(), &ShimFormat::EsModule);
	assert_eq!(
		WorkerBuilder::new_with_url(&url).unwrap_err(),
		ModuleSupportError
	);
}
