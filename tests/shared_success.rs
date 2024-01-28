#![cfg(target_family = "wasm")]

mod basic_success;
// Firefox doesn't support blocking in shared workers.
// See <https://bugzilla.mozilla.org/show_bug.cgi?id=1359745>.
#[cfg(not(unsupported_shared_block))]
mod supported_block;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_shared_worker);
