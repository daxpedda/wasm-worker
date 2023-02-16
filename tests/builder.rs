//! Tests functionality around [`WorkerBuilder`].

mod util;

use anyhow::Result;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::dedicated::{ShimFormat, WorkerBuilder, WorkerUrl};

use self::util::Flag;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`WorkerBuilder::spawn()`].
#[wasm_bindgen_test]
async fn spawn() -> Result<()> {
	let flag = Flag::new();

	WorkerBuilder::new()?.spawn({
		let flag = flag.clone();
		move |context| {
			flag.signal();
			context.close();
		}
	});

	flag.await;

	Ok(())
}

/// [`WorkerBuilder::spawn_async()`].
#[wasm_bindgen_test]
async fn spawn_async() -> Result<()> {
	let flag = Flag::new();

	WorkerBuilder::new()?.spawn_async({
		let flag = flag.clone();
		|context| async move {
			flag.signal();
			context.close();
		}
	});

	flag.await;

	Ok(())
}

/// [`WorkerBuilder::new_with_url()`].
#[wasm_bindgen_test]
async fn url() -> Result<()> {
	let flag = Flag::new();

	// Ideally we would built a custom JS that can receive an atomic.
	// Instead we will just use the regular `WorkerUrl` but build it ourselves.
	let url = WorkerUrl::new(
		&wasm_bindgen::shim_url().unwrap(),
		match &wasm_bindgen::shim_format().unwrap() {
			wasm_bindgen::ShimFormat::EsModule => ShimFormat::EsModule,
			wasm_bindgen::ShimFormat::NoModules { global_name } => ShimFormat::Classic {
				global: global_name,
			},
			_ => unreachable!("expected shim to be built for browsers"),
		},
	);

	WorkerBuilder::new_with_url(&url)?.spawn({
		let flag = flag.clone();
		move |context| {
			flag.signal();
			context.close();
		}
	});

	flag.await;

	Ok(())
}

/// [`WorkerBuilder::name()`].
#[wasm_bindgen_test]
async fn name() -> Result<()> {
	let flag = Flag::new();

	WorkerBuilder::new()?.name("test").spawn({
		let flag = flag.clone();
		move |context| {
			assert_eq!(context.name(), Some(String::from("test")));
			// Flag will never signal if `assert!` panics.
			flag.signal();

			context.close();
		}
	});

	flag.await;

	Ok(())
}
