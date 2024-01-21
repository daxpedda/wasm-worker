#![cfg(target_family = "wasm")]
#![cfg(not(skip_service))]

mod basic;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_service_worker);
