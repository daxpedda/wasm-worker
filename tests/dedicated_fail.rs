#![cfg(target_family = "wasm")]

#[cfg(any(not(target_feature = "atomics"), unsupported_spawn))]
mod unsupported_spawn;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_dedicated_worker);
