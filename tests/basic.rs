#![allow(unreachable_pub)]

mod util;

use std::time::Duration;

use futures_util::future::{self, Either};
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::Close;

use self::util::Flag;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn basic() {
	let flag = Flag::new();

	wasm_worker::spawn({
		let flag = flag.clone();
		move |_| async move {
			flag.signal();
			Close::Yes
		}
	});

	flag.await;
}

#[wasm_bindgen_test]
async fn nested() {
	let outer_flag = Flag::new();

	wasm_worker::spawn({
		let outer_flag = outer_flag.clone();
		move |_| async move {
			let inner_flag = Flag::new();

			wasm_worker::spawn({
				let inner_flag = inner_flag.clone();
				move |_| async move {
					inner_flag.signal();
					Close::Yes
				}
			});

			inner_flag.await;

			// Wait for nested worker to close.
			// See <https://bugs.chromium.org/p/chromium/issues/detail?id=1408115>.
			util::sleep(Duration::from_millis(250)).await;

			outer_flag.signal();

			Close::Yes
		}
	});

	outer_flag.await;
}

#[wasm_bindgen_test]
async fn non_closing() {
	let signal_flag = Flag::new();
	let response_flag = Flag::new();

	let worker = wasm_worker::spawn({
		let signal_flag = signal_flag.clone();
		let response_flag = response_flag.clone();
		move |_| async move {
			wasm_bindgen_futures::spawn_local(async move {
				signal_flag.await;
				response_flag.signal();
			});

			Close::No
		}
	});

	signal_flag.signal();
	response_flag.await;
	worker.terminate();
}

#[wasm_bindgen_test]
async fn terminate() {
	let signal_flag = Flag::new();
	let response_flag = Flag::new();

	let worker = wasm_worker::spawn({
		let signal_flag = signal_flag.clone();
		let response_flag = response_flag.clone();
		move |_| async move {
			signal_flag.await;
			response_flag.signal();

			Close::Yes
		}
	});

	worker.terminate();
	signal_flag.signal();

	let result = future::select(response_flag, util::sleep(Duration::from_millis(250))).await;
	assert!(matches!(result, Either::Right(((), _))));
}