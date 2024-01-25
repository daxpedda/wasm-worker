#![cfg(target_family = "wasm")]

mod basic_fail;
mod unsupported_spawn;
// Firefox doesn't support waiting in shared workers.
// See <https://bugzilla.mozilla.org/show_bug.cgi?id=1359745>.
#[cfg(unsupported_shared_wait)]
mod unsupported_wait;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_shared_worker);
