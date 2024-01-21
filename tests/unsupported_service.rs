#![cfg(target_family = "wasm")]
#![cfg(not(service_unsupported))]

mod unsupported_spawn;
mod unsupported_wait;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_service_worker);
