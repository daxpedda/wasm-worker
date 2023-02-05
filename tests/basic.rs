#![allow(unreachable_pub)]

mod util;

use std::time::Duration;

use futures_util::future::{self, Either};
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::{Close, Message, WorkerBuilder};

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

#[wasm_bindgen_test]
async fn array_buffer_transfer() -> Result<(), JsValue> {
	let buffer = ArrayBuffer::new(1);
	let array = Uint8Array::new(&buffer);
	array.copy_from(&[42]);

	let flag_start = Flag::new();
	let flag_sent = Flag::new();
	let flag_finish = Flag::new();

	let worker = WorkerBuilder::new()?
		.set_message_handler({
			let flag_finish = flag_finish.clone();
			move |event| {
				if let Some(Message::ArrayBuffer(buffer)) = event.message() {
					let array = Uint8Array::new(&buffer);
					assert_eq!(array.get_index(0), 42);
				} else {
					panic!()
				}

				flag_finish.signal();
			}
		})
		.spawn({
			let flag_start = flag_start.clone();
			let flag_sent = flag_sent.clone();
			|context| async move {
				context.set_message_handler(move |context, event| {
					if let Some(Message::ArrayBuffer(buffer)) = event.message() {
						let array = Uint8Array::new(&buffer);
						assert_eq!(array.get_index(0), 42);

						assert!(context.transfer_message(buffer.into()).is_ok());
					} else {
						panic!()
					}

					flag_sent.signal();
				});

				flag_start.signal();

				Close::No
			}
		});

	flag_start.await;

	assert!(worker
		.transfer_message(Message::ArrayBuffer(buffer))
		.is_ok());

	flag_sent.await;
	flag_finish.await;

	worker.terminate();

	Ok(())
}
