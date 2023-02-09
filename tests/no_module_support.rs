use wasm_bindgen::ShimFormat;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::{ModuleSupportError, WorkerBuilder, WorkerUrl, WorkerUrlFormat};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn assert_target() {
	assert!(matches!(
		wasm_bindgen::shim_format(),
		Some(ShimFormat::EsModule)
	));
}

#[wasm_bindgen_test]
#[should_panic(expected = "ModuleSupportError")]
fn spawn() {
	wasm_worker::spawn(|_| unreachable!());
}

#[wasm_bindgen_test]
#[should_panic(expected = "ModuleSupportError")]
fn spawn_async() {
	wasm_worker::spawn_async(|_| async { unreachable!() });
}

#[wasm_bindgen_test]
fn check() {
	assert!(!WorkerBuilder::has_module_support());
}

#[wasm_bindgen_test]
fn builder() {
	assert_eq!(WorkerBuilder::new().unwrap_err(), ModuleSupportError);
}

#[wasm_bindgen_test]
fn builder_url() {
	let url = WorkerUrl::new(
		&wasm_bindgen::shim_url().unwrap(),
		WorkerUrlFormat::EsModule,
	);
	assert_eq!(
		WorkerBuilder::new_with_url(&url).unwrap_err(),
		ModuleSupportError
	);
}
