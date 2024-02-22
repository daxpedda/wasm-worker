#[cfg(target_family = "wasm")]
mod basic_success;
#[cfg(all(
	target_family = "wasm",
	any(not(unsupported_spawn), not(unsupported_wait_async))
))]
mod basic_success_async;
mod supported_block;
#[cfg(all(
	target_family = "wasm",
	target_feature = "atomics",
	not(unsupported_spawn)
))]
mod supported_spawn_success;

#[cfg(target_family = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_dedicated_worker);
