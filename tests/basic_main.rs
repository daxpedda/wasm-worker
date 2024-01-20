#![cfg(test)]

mod basic;
#[cfg(any(
	not(target_family = "wasm"),
	all(target_family = "wasm", target_feature = "atomics")
))]
mod spawn;

#[cfg(target_family = "wasm")]
use wasm_bindgen_test::wasm_bindgen_test;

#[cfg(target_family = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
fn park_no_op() {
	use web_time::Duration;

	web_thread::park();
	web_thread::park_timeout(Duration::from_secs(1));
	#[allow(deprecated)]
	web_thread::park_timeout_ms(1000);
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
#[allow(clippy::absolute_paths)]
fn has_wait_support() {
	assert!(!web_thread::web::has_wait_support());
}
