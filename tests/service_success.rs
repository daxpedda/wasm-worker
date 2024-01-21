#![cfg(target_family = "wasm")]
#![cfg(not(unsupported_service))]

mod supported_basic;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_service_worker);
