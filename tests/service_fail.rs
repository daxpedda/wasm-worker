#![cfg(target_family = "wasm")]
// Firefox doesn't support module service workers.
// See <https://bugzilla.mozilla.org/show_bug.cgi?id=1360870>.
#![cfg(not(unsupported_service))]

mod basic_fail;
mod unsupported_block;
mod unsupported_spawn;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_service_worker);
