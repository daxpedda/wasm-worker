//! Tests functionality around [`WorkerBuilder`].

mod util;

use wasm_bindgen::{JsValue, ShimFormat};
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::{Close, WorkerBuilder, WorkerUrl, WorkerUrlFormat};

use self::util::Flag;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`WorkerBuilder::spawn()`].
#[wasm_bindgen_test]
async fn spawn() -> Result<(), JsValue> {
	let flag = Flag::new();

	WorkerBuilder::new()?.spawn({
		let flag = flag.clone();
		move |_| {
			flag.signal();
			Close::Yes
		}
	});

	flag.await;

	Ok(())
}

/// [`WorkerBuilder::spawn_async()`].
#[wasm_bindgen_test]
async fn spawn_async() -> Result<(), JsValue> {
	let flag = Flag::new();

	WorkerBuilder::new()?.spawn_async({
		let flag = flag.clone();
		|_| async move {
			flag.signal();
			Close::Yes
		}
	});

	flag.await;

	Ok(())
}

/// [`WorkerBuilder::new_with_url()`].
#[wasm_bindgen_test]
async fn url() -> Result<(), JsValue> {
	let flag = Flag::new();

	// Ideally we would built a custom JS that can receive an atomic.
	// Instead we will just use the regular `WorkerUrl` but build it ourselves.
	let url = WorkerUrl::new(
		&wasm_bindgen::shim_url().unwrap(),
		match &wasm_bindgen::shim_format().unwrap() {
			ShimFormat::EsModule => WorkerUrlFormat::EsModule,
			ShimFormat::NoModules { global_name } => WorkerUrlFormat::Classic {
				global: global_name,
			},
			_ => unreachable!("expected shim to be built for browsers"),
		},
	);

	WorkerBuilder::new_with_url(&url)?.spawn({
		let flag = flag.clone();
		move |_| {
			flag.signal();
			Close::Yes
		}
	});

	flag.await;

	Ok(())
}

/// [`WorkerBuilder::name()`].
#[wasm_bindgen_test]
async fn name() -> Result<(), JsValue> {
	let flag = Flag::new();

	WorkerBuilder::new()?.name("test").spawn({
		let flag = flag.clone();
		move |context| {
			assert_eq!(context.name(), Some(String::from("test")));
			// Flag will never signal if `assert!` panics.
			flag.signal();

			Close::Yes
		}
	});

	flag.await;

	Ok(())
}
