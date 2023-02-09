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

/// Tests transfering `T`.
///
/// If `force` is `true` the test will fail if the type is not supported,
/// otherwise the test will be skipped.
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

	let request = Flag::new();
	let response = Flag::new();

	let worker = WorkerBuilder::new()?
		.message_handler({
			let response = response.clone();
			move |_, event| {
				let value: T = event.messages().next().unwrap().serialize_as().unwrap();

				assert(&value);

				response.signal();
			}
		})
		.spawn({
			let request = request.clone();
			move |context| {
				context.set_message_handler(move |context, event| {
					let value: T = event.messages().next().unwrap().serialize_as().unwrap();

					assert(&value);

					context.transfer_messages([value]);
				});

				request.signal();

				Close::No
			}
		});

	request.await;
	worker.transfer_messages([value]);
	response.await;

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
