mod basic;

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
