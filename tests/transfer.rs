#![allow(unreachable_pub)]

mod util;

use std::time::Duration;

use futures_util::{future, FutureExt};
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::{Close, Message, WorkerBuilder};
#[cfg(web_sys_unstable_apis)]
use {
	wasm_bindgen::UnwrapThrowExt,
	web_sys::{AudioData, AudioDataCopyToOptions, AudioDataInit, AudioSampleFormat},
};

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

async fn test_transfer<T: JsCast + Into<Message>>(
	support: impl FnOnce() -> bool,
	force: bool,
	init: impl FnOnce() -> T,
	assert: impl 'static + Copy + Fn(&T) + Send,
) -> Result<(), JsValue> {
	if !support() {
		if force {
			panic!("type unsupported in this browser")
		} else {
			return Ok(());
		}
	}

	let value = init();

	let flag_start = Flag::new();
	let flag_finish = Flag::new();

	let worker = WorkerBuilder::new()?
		.set_message_handler({
			let flag_finish = flag_finish.clone();
			move |_, event| {
				let value = event
					.messages()
					.next()
					.unwrap()
					.serialize_as::<T>()
					.unwrap();

				assert(&value);

				flag_finish.signal();
			}
		})
		.spawn({
			let flag_start = flag_start.clone();
			move |context| {
				context.set_message_handler(move |context, event| {
					let value = event
						.messages()
						.next()
						.unwrap()
						.serialize_as::<T>()
						.unwrap();

					assert(&value);

					context.transfer_messages([value]);
				});

				flag_start.signal();

				Close::No
			}
		});

	flag_start.await;
	worker.transfer_message([value]);
	flag_finish.await;

	worker.terminate();

	Ok(())
}

#[wasm_bindgen_test]
async fn array_buffer() -> Result<(), JsValue> {
	test_transfer(
		Message::has_array_buffer_support,
		true,
		|| {
			let buffer = ArrayBuffer::new(1);
			let array = Uint8Array::new(&buffer);
			array.copy_from(&[42]);
			buffer
		},
		|buffer| {
			let array = Uint8Array::new(buffer);
			assert_eq!(array.get_index(0), 42);
		},
	)
	.await
}

#[wasm_bindgen_test]
#[cfg(web_sys_unstable_apis)]
async fn audio_data() -> Result<(), JsValue> {
	test_transfer(
		Message::has_audio_data_support,
		false,
		|| {
			let init = AudioDataInit::new(
				&ArrayBuffer::new(42),
				AudioSampleFormat::U8,
				1,
				42,
				3000.,
				0.,
			);
			AudioData::new(&init).unwrap_throw()
		},
		|data| {
			let size = data.allocation_size(&AudioDataCopyToOptions::new(0));
			assert_eq!(size, 42);
		},
	)
	.await
}
