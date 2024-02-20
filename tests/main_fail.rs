#![cfg(target_family = "wasm")]

mod audio_worklet_fail;
mod basic_fail;
#[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
mod supported_spawn_fail;
mod unsupported_block;
#[cfg(any(not(target_feature = "atomics"), unsupported_spawn))]
mod unsupported_spawn;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
