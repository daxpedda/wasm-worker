#![cfg(target_family = "wasm")]

mod basic;
#[cfg(target_feature = "atomics")]
mod wait;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_shared_worker);
