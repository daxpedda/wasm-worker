#![allow(unreachable_pub)]

mod util;

use std::time::Duration;

use futures_util::future::{self, Either};
use futures_util::FutureExt;
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

	worker.transfer_messages([buffer]);

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

#[wasm_bindgen_test]
async fn multi_messages() -> Result<(), JsValue> {
	assert!(Message::has_array_buffer_support());

	let flag = Flag::new();

	let worker = WorkerBuilder::new()?
		.set_message_handler(move |context, event| {
			let messages: Vec<ArrayBuffer> = event
				.messages()
				.map(|message| message.serialize_as().unwrap())
				.collect();

			assert_eq!(messages.len(), 3);

			for (index, buffer) in (1_u8..).zip(&messages) {
				let array = Uint8Array::new(buffer);
				assert_eq!(buffer.byte_length(), index.into());
				let mut output = [0; 3];
				array.copy_to(&mut output[..index.into()]);
				assert!(output[..index.into()].iter().all(|value| *value == index));
			}

			context.transfer_messages(messages);
		})
		.spawn_async({
			let flag = flag.clone();
			|context| async move {
				context.set_message_handler(move |_, event| {
					let messages = event.messages();
					assert_eq!(messages.len(), 3);

					for (index, message) in (1_u8..).zip(messages) {
						let buffer: ArrayBuffer = message.serialize_as().unwrap();
						let array = Uint8Array::new(&buffer);
						assert_eq!(buffer.byte_length(), index.into());
						let mut output = [0; 3];
						array.copy_to(&mut output[..index.into()]);
						assert!(output[..index.into()].iter().all(|value| *value == index));
					}

					flag.signal();
				});

				let buffer_1 = ArrayBuffer::new(1);
				let array = Uint8Array::new(&buffer_1);
				array.copy_from(&[1]);

				let buffer_2 = ArrayBuffer::new(2);
				let array = Uint8Array::new(&buffer_2);
				array.copy_from(&[2; 2]);

				let buffer_3 = ArrayBuffer::new(3);
				let array = Uint8Array::new(&buffer_3);
				array.copy_from(&[3; 3]);

				context.transfer_messages([buffer_1, buffer_2, buffer_3]);

				Close::No
			}
		});

	flag.await;

	worker.terminate();

	Ok(())
}

#[wasm_bindgen_test]
async fn async_message_handler() {
	assert!(Message::has_array_buffer_support());

	let flag = Flag::new();

	let worker = wasm_worker::spawn(|context| {
		let buffer = ArrayBuffer::new(1);
		context.transfer_messages([buffer]);

		Close::Yes
	});

	worker.set_message_handler_async({
		let flag = flag.clone();
		move |_, _| {
			let flag = flag.clone();
			async move {
				util::sleep(Duration::from_millis(250)).await;
				flag.signal();
			}
		}
	});

	let result = future::select(flag, util::sleep(Duration::from_millis(500))).await;
	assert!(matches!(result, Either::Left(((), _))));
}

#[wasm_bindgen_test]
async fn abort_async_message_handler() {
	assert!(Message::has_array_buffer_support());

	let mut received_1 = Flag::new();
	let received_1_broken = Flag::new();
	let start_2 = Flag::new();
	let received_2 = Flag::new();

	let worker = wasm_worker::spawn_async({
		let start_2 = start_2.clone();
		|context| async move {
			let buffer = ArrayBuffer::new(1);
			context.transfer_messages([buffer]);

			start_2.await;

			let buffer = ArrayBuffer::new(1);
			context.transfer_messages([buffer]);

			Close::Yes
		}
	});

	worker.set_message_handler_async({
		let received_1 = received_1.clone();
		let received_1_broken = received_1_broken.clone();

		move |_, _| {
			let received_1 = received_1.clone();
			let received_1_broken = received_1_broken.clone();
			async move {
				received_1.signal();
				util::sleep(Duration::from_millis(250)).await;
				received_1_broken.signal();
			}
		}
	});

	(&mut received_1).await;

	worker.set_message_handler_async({
		let received_2 = received_2.clone();
		move |_, _| {
			let received_2 = received_2.clone();
			async move {
				util::sleep(Duration::from_millis(500)).await;
				received_2.signal();
			}
		}
	});

	start_2.signal();

	let result = future::select_all([
		received_1_broken.map(|_| false).boxed_local(),
		received_2.map(|_| true).boxed_local(),
		util::sleep(Duration::from_millis(750))
			.map(|_| false)
			.boxed_local(),
	])
	.await;
	assert!(result.0);
}
