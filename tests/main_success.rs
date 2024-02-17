#[cfg(all(
	target_family = "wasm",
	target_feature = "atomics",
	feature = "audio-worklet",
	not(unsupported_spawn)
))]
mod audio_worklet_success;
mod basic_success;
#[cfg(any(
	not(target_family = "wasm"),
	all(
		target_family = "wasm",
		target_feature = "atomics",
		not(unsupported_spawn)
	)
))]
mod supported_spawn_success;

#[cfg(target_family = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
