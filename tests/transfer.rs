#![allow(unreachable_pub)]

mod util;

use std::time::Duration;

use futures_util::{future, FutureExt};
use js_sys::{ArrayBuffer, Uint8Array};
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

	let mut worker = WorkerBuilder::new()?
		.set_message_handler({
			let flag_received = flag_spawner_received.clone();
			move |_| flag_received.signal()
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
				context.transfer_messages([buffer.into()]);

				Close::No
			}
		});

	worker.clear_message_handler();

	flag_spawner_start.signal();
	flag_worker_start.await;

	worker.transfer_message([buffer.into()]);

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
async fn array_buffer() -> Result<(), JsValue> {
	assert!(Message::has_array_buffer_support());

	let buffer = ArrayBuffer::new(1);
	let array = Uint8Array::new(&buffer);
	array.copy_from(&[42]);

	let flag_start = Flag::new();
	let flag_finish = Flag::new();

	let worker = WorkerBuilder::new()?
		.set_message_handler({
			let flag_finish = flag_finish.clone();
			move |event| {
				if let Ok(Message::ArrayBuffer(buffer)) = event
					.messages()
					.next()
					.unwrap()
					.serialize_as::<ArrayBuffer>()
				{
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
			move |context| {
				context.set_message_handler(move |context, event| {
					if let Ok(Message::ArrayBuffer(buffer)) = event
						.messages()
						.next()
						.unwrap()
						.serialize_as::<ArrayBuffer>()
					{
						let array = Uint8Array::new(&buffer);
						assert_eq!(array.get_index(0), 42);

						context.transfer_messages([buffer.into()]);
					} else {
						panic!()
					}
				});

				flag_start.signal();

				Close::No
			}
		});

	flag_start.await;
	worker.transfer_message([buffer.into()]);
	flag_finish.await;

	worker.terminate();

	Ok(())
}
