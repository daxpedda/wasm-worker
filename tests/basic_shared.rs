#![cfg(target_family = "wasm")]

mod basic;
#[cfg(all(target_feature = "atomics", not(shared_unsupported_wait)))]
mod wait;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_shared_worker);
