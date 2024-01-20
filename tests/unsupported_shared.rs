#![cfg(target_family = "wasm")]

mod unsupported_spawn;
#[cfg(not(target_feature = "atomics"))]
mod unsupported_wait;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_shared_worker);
