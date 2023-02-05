#![allow(unreachable_pub)]

mod util;

use wasm_bindgen::{JsValue, ShimFormat};
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::{Close, WorkerBuilder, WorkerUrl, WorkerUrlFormat};

use self::util::Flag;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn basic() -> Result<(), JsValue> {
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

#[wasm_bindgen_test]
async fn url() -> Result<(), JsValue> {
	let flag = Flag::new();

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

#[wasm_bindgen_test]
async fn name() -> Result<(), JsValue> {
	let flag = Flag::new();

	WorkerBuilder::new()?.name("test").spawn({
		let flag = flag.clone();
		move |context| {
			assert_eq!(context.name(), Some(String::from("test")));

			flag.signal();
			Close::Yes
		}
	});

	flag.await;

	Ok(())
}

#[wasm_bindgen_test]
async fn clear_name() -> Result<(), JsValue> {
	let flag = Flag::new();

	WorkerBuilder::new()?.name("test").clear_name().spawn({
		let flag = flag.clone();
		move |context| {
			assert_eq!(context.name(), None);

			flag.signal();
			Close::Yes
		}
	});

	flag.await;

	Ok(())
}
