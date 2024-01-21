#![cfg(target_family = "wasm")]

mod supported_basic;
#[cfg(all(target_feature = "atomics", not(unsupported_shared_wait)))]
mod supported_wait;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_shared_worker);
