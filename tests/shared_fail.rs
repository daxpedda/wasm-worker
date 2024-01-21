#![cfg(target_family = "wasm")]

mod unsupported_spawn;
#[cfg(unsupported_shared_wait)]
mod unsupported_wait;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_shared_worker);
