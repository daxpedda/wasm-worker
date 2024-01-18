#![cfg(test)]
#![cfg(any(not(target_family = "wasm"), target_feature = "atomics"))]

#[cfg(not(target_family = "wasm"))]
use std::time;
use std::time::Duration;

#[cfg(target_family = "wasm")]
use thread::web::JoinHandleExt;
use time::Instant;
use web_thread as thread;
#[cfg(target_family = "wasm")]
use web_time as time;

#[cfg(target_family = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test::wasm_bindgen_test)]
fn available_parallelism() {
	thread::available_parallelism().unwrap();
}

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test::wasm_bindgen_test)]
fn thread() {
	let thread = thread::current();
	let _ = thread.id();
	let _ = thread.name();
	thread.unpark();
}

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test::wasm_bindgen_test)]
fn panicking() {
	assert!(!thread::panicking());
}

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test::wasm_bindgen_test)]
async fn park() {
	let start = Instant::now();

	#[cfg_attr(not(target_family = "wasm"), allow(unused_mut))]
	let mut handle = thread::spawn(|| {
		thread::park();
		thread::park_timeout(Duration::from_secs(1));
		#[allow(deprecated)]
		thread::park_timeout_ms(1000);
	});

	handle.thread().unpark();
	handle.thread().unpark();
	handle.thread().unpark();

	#[cfg(not(target_family = "wasm"))]
	handle.join().unwrap();
	#[cfg(target_family = "wasm")]
	handle.join_async().await.unwrap();

	assert!(start.elapsed().as_secs() >= 2);
}

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test::wasm_bindgen_test)]
async fn sleep() {
	let start = Instant::now();

	#[cfg_attr(not(target_family = "wasm"), allow(unused_mut))]
	let mut handle = thread::spawn(|| {
		thread::sleep(Duration::from_secs(1));
		#[allow(deprecated)]
		thread::sleep_ms(1000);
	});

	#[cfg(not(target_family = "wasm"))]
	handle.join().unwrap();
	#[cfg(target_family = "wasm")]
	handle.join_async().await.unwrap();

	assert!(start.elapsed().as_secs() >= 2);
}
