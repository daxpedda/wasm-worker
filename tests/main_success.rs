mod supported_basic;
#[cfg(any(
	not(target_family = "wasm"),
	all(
		target_family = "wasm",
		target_feature = "atomics",
		not(unsupported_spawn)
	)
))]
mod supported_spawn;

#[cfg(target_family = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
