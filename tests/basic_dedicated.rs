#[cfg(target_family = "wasm")]
mod basic;
#[cfg(all(target_family = "wasm", target_feature = "atomics"))]
mod spawn;
mod wait;

#[cfg(target_family = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_dedicated_worker);
