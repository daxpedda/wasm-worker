use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker_core::{Error, WorkerBuilder};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn no_module_support() {
	assert!(matches!(
		WorkerBuilder::new().unwrap_err(),
		Error::NoModuleSupport(_)
	));
}
