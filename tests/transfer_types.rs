#![allow(unreachable_pub)]

mod util;

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
	worker.transfer_messages([value]);
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
