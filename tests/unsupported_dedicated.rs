#![cfg(test)]
#![cfg(target_family = "wasm")]

#[cfg(not(target_feature = "atomics"))]
mod unsupported_spawn;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_dedicated_worker);
