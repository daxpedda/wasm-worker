#![cfg(test)]

#[cfg(not(target_family = "wasm"))]
use std::time;

#[cfg(any(
	not(target_family = "wasm"),
	all(target_family = "wasm", target_feature = "atomics")
))]
use time::{Duration, Instant};
#[cfg(target_family = "wasm")]
use wasm_bindgen_test::wasm_bindgen_test;
#[cfg(all(target_family = "wasm", target_feature = "atomics"))]
use web_thread::web::JoinHandleExt;
#[cfg(all(target_family = "wasm", target_feature = "atomics"))]
use web_time as time;

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn available_parallelism() {
	web_thread::available_parallelism().unwrap();
}

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn thread() {
	let thread = web_thread::current();
	let _ = thread.id();
	let _ = thread.name();
	thread.unpark();
}

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn panicking() {
	assert!(!web_thread::panicking());
}

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
#[cfg(any(
	not(target_family = "wasm"),
	all(target_family = "wasm", target_feature = "atomics")
))]
async fn park() {
	let start = Instant::now();

	#[cfg_attr(not(target_family = "wasm"), allow(unused_mut))]
	let mut handle = web_thread::spawn(|| {
		web_thread::park();
		web_thread::park_timeout(Duration::from_secs(1));
		#[allow(deprecated)]
		web_thread::park_timeout_ms(1000);
	});

	handle.thread().unpark();

	#[cfg(not(target_family = "wasm"))]
	handle.join().unwrap();
	#[cfg(target_family = "wasm")]
	handle.join_async().await.unwrap();

	assert!(start.elapsed().as_secs() >= 2);
}

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
#[cfg(any(
	not(target_family = "wasm"),
	all(target_family = "wasm", target_feature = "atomics")
))]
async fn sleep() {
	let start = Instant::now();

	#[cfg_attr(not(target_family = "wasm"), allow(unused_mut))]
	let mut handle = web_thread::spawn(|| {
		web_thread::sleep(Duration::from_secs(1));
		#[allow(deprecated)]
		web_thread::sleep_ms(1000);
	});

	#[cfg(not(target_family = "wasm"))]
	handle.join().unwrap();
	#[cfg(target_family = "wasm")]
	handle.join_async().await.unwrap();

	assert!(start.elapsed().as_secs() >= 2);
}
