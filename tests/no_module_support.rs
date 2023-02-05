use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::{ModuleSupportError, WorkerBuilder};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn check() {
	assert!(!WorkerBuilder::has_module_support());
}

#[wasm_bindgen_test]
fn builder_error() {
	assert_eq!(WorkerBuilder::new().unwrap_err(), ModuleSupportError);
}
