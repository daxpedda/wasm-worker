#![cfg(test)]

#[cfg(not(target_family = "wasm"))]
use std::time;

#[cfg(target_family = "wasm")]
use wasm_bindgen_test::wasm_bindgen_test;
#[cfg(any(
	not(target_family = "wasm"),
	all(target_family = "wasm", target_feature = "atomics")
))]
use {
	time::{Duration, Instant},
	web_thread::Builder,
};
#[cfg(all(target_family = "wasm", target_feature = "atomics"))]
use {web_thread::web::JoinHandleExt, web_time as time};

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

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
#[cfg(any(
	not(target_family = "wasm"),
	all(target_family = "wasm", target_feature = "atomics")
))]
async fn builder() {
	#[cfg_attr(not(target_family = "wasm"), allow(unused_mut))]
	let mut handle = Builder::new()
		.stack_size(0)
		.spawn(|| assert_eq!(web_thread::current().name(), None))
		.unwrap();

	#[cfg(not(target_family = "wasm"))]
	handle.join().unwrap();
	#[cfg(target_family = "wasm")]
	handle.join_async().await.unwrap();
}

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
#[cfg(any(
	not(target_family = "wasm"),
	all(target_family = "wasm", target_feature = "atomics")
))]
async fn builder_name() {
	#[cfg_attr(not(target_family = "wasm"), allow(unused_mut))]
	let mut handle = Builder::new()
		.stack_size(0)
		.name(String::from("test"))
		.spawn(|| assert_eq!(web_thread::current().name(), Some("test")))
		.unwrap();

	#[cfg(not(target_family = "wasm"))]
	handle.join().unwrap();
	#[cfg(target_family = "wasm")]
	handle.join_async().await.unwrap();
}

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
#[cfg(any(
	not(target_family = "wasm"),
	all(target_family = "wasm", target_feature = "atomics")
))]
async fn is_finished() {
	#[cfg_attr(not(target_family = "wasm"), allow(unused_mut))]
	let mut handle = web_thread::spawn(|| {
		web_thread::park();
	});

	assert!(!handle.is_finished());

	handle.thread().unpark();

	#[cfg(not(target_family = "wasm"))]
	handle.join().unwrap();
	#[cfg(target_family = "wasm")]
	handle.join_async().await.unwrap();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
#[allow(clippy::absolute_paths)]
fn has_thread_support() {
	assert!(web_thread::web::has_spawn_support());
}
