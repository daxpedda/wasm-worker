//! Tests functionality around [`WorkerBuilder`].

#![allow(clippy::missing_assert_message)]

mod util;

use wasm_bindgen_test::wasm_bindgen_test;
use web_thread::WorkerBuilder;

use self::util::Flag;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`WorkerBuilder::spawn()`].
#[wasm_bindgen_test]
async fn spawn() {
	let flag = Flag::new();

	WorkerBuilder::new().spawn({
		let flag = flag.clone();
		move |context| {
			flag.signal();
			context.close();
		}
	});

	flag.await;
}

/// [`WorkerBuilder::spawn_async()`].
#[wasm_bindgen_test]
async fn spawn_async() {
	let flag = Flag::new();

	WorkerBuilder::new().spawn_async({
		let flag = flag.clone();
		|context| async move {
			flag.signal();
			context.close();
		}
	});

	flag.await;
}

/// [`WorkerBuilder::name()`].
#[wasm_bindgen_test]
async fn name() {
	let flag = Flag::new();

	WorkerBuilder::new().name("test").spawn({
		let flag = flag.clone();
		move |context| {
			assert_eq!(context.name(), Some(String::from("test")));
			// Flag will never signal if `assert!` panics.
			flag.signal();

			context.close();
		}
	});

	flag.await;
}
