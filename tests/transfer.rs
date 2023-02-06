#![allow(unreachable_pub)]

mod util;

use std::time::Duration;

use futures_util::{future, FutureExt};
use js_sys::ArrayBuffer;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::{Close, Message, WorkerBuilder};

use self::util::Flag;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn clear_message_handler() -> Result<(), JsValue> {
	assert!(Message::has_array_buffer_support());

	let buffer = ArrayBuffer::new(1);

	let flag_spawner_start = Flag::new();
	let flag_worker_start = Flag::new();
	let flag_spawner_received = Flag::new();
	let flag_worker_received = Flag::new();

	let worker = WorkerBuilder::new()?
		.set_message_handler({
			let flag_received = flag_spawner_received.clone();
			move |_, _| flag_received.signal()
		})
		.spawn_async({
			let flag_spawner_start = flag_spawner_start.clone();
			let flag_worker_start = flag_worker_start.clone();
			let flag_received = flag_worker_received.clone();
			|context| async move {
				context.set_message_handler(move |_, _| flag_received.signal());
				context.clear_message_handler();

				flag_worker_start.signal();
				flag_spawner_start.await;

				let buffer = ArrayBuffer::new(1);
				context.transfer_messages([buffer]);

				Close::No
			}
		});

	worker.clear_message_handler();

	flag_spawner_start.signal();
	flag_worker_start.await;

	worker.transfer_message([buffer]);

	let result = future::select_all([
		flag_spawner_received.map(|_| true).boxed_local(),
		flag_worker_received.map(|_| true).boxed_local(),
		util::sleep(Duration::from_millis(250))
			.map(|_| false)
			.boxed_local(),
	])
	.await;
	assert!(!result.0);

	worker.terminate();

	Ok(())
}

#[wasm_bindgen_test]
fn has_message_handler() -> Result<(), JsValue> {
	let worker = WorkerBuilder::new()?
		.set_message_handler(|_, _| ())
		.spawn(|_| Close::Yes);
	assert!(worker.has_message_handler());
	worker.clear_message_handler();
	assert!(!worker.has_message_handler());

	let worker = WorkerBuilder::new()?
		.set_message_handler(|_, _| ())
		.clear_message_handler()
		.spawn(|_| Close::Yes);
	assert!(!worker.has_message_handler());

	let worker = wasm_worker::spawn(|_| Close::Yes);

	assert!(!worker.has_message_handler());
	worker.set_message_handler(|_, _| ());
	assert!(worker.has_message_handler());
	worker.clear_message_handler();
	assert!(!worker.has_message_handler());

	Ok(())
}
