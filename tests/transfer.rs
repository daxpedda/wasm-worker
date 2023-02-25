//! Test each transferable type to be sent and received successfully. This is
//! important to assert that the support methods of [`Message`] are correct.

mod util;

use std::fmt::Debug;
use std::future::{ready, Future};

use anyhow::Result;
use futures_util::future::{self, Either};
use futures_util::FutureExt;
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::dedicated::WorkerBuilder;
use wasm_worker::message::{Message, SupportError};
#[cfg(web_sys_unstable_apis)]
use web_sys::{
	AudioData, AudioDataCopyToOptions, AudioDataInit, AudioSampleFormat, VideoFrame,
	VideoFrameBufferInit, VideoPixelFormat,
};
use web_sys::{
	ImageBitmap, ImageData, MessageChannel, MessagePort, OffscreenCanvas, ReadableStream,
	RtcDataChannel, RtcDataChannelState, RtcPeerConnection, TransformStream, WritableStream,
};

use self::util::{Flag, SIGNAL_DURATION};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`RawMessage::serialize()`](wasm_worker::RawMessage::serialize).
#[wasm_bindgen_test]
async fn serialize() -> Result<()> {
	assert!(Message::has_array_buffer_support().is_ok());

	let flag = Flag::new();

	let _worker = WorkerBuilder::new()?
		.message_handler({
			let flag = flag.clone();
			move |_, event| {
				let mut message = event.messages().unwrap().into_iter().next().unwrap();

				#[cfg(web_sys_unstable_apis)]
				{
					message = message.serialize_as::<AudioData>().unwrap_err().0;
				}
				message = message.serialize_as::<ImageBitmap>().unwrap_err().0;
				message = message.serialize_as::<MessagePort>().unwrap_err().0;
				message = message.serialize_as::<OffscreenCanvas>().unwrap_err().0;
				message = message.serialize_as::<ReadableStream>().unwrap_err().0;
				message = message.serialize_as::<RtcDataChannel>().unwrap_err().0;
				message = message.serialize_as::<TransformStream>().unwrap_err().0;
				#[cfg(web_sys_unstable_apis)]
				{
					message = message.serialize_as::<VideoFrame>().unwrap_err().0;
				}
				message.serialize_as::<WritableStream>().unwrap_err();

				flag.signal();
			}
		})
		.spawn({
			|context| {
				context.transfer_messages([ArrayBuffer::new(1)]).unwrap();

				context.close();
			}
		});

	flag.await;

	Ok(())
}

/// Tests transfering `T`.
///
/// If `force` is `true` the test will fail if the type is not supported,
/// otherwise the test will be skipped.
async fn test_transfer<T, F1, F2, F3>(
	support: impl Fn() -> F1,
	force: bool,
	init: impl Fn() -> F2,
	assert_sent: impl 'static + Copy + Fn(&T) + Send,
	assert_received: impl 'static + Clone + Fn(&T) -> F3 + Send,
) -> Result<()>
where
	T: Clone + JsCast + TryFrom<Message>,
	Message: From<T>,
	<T as TryFrom<Message>>::Error: Debug,
	F1: Future<Output = Result<(), SupportError>>,
	F2: Future<Output = T>,
	F3: Future<Output = ()>,
{
	let message = Message::from(JsValue::UNDEFINED.unchecked_into());
	let mut future = message.has_support();
	assert!(future.into_inner().is_some());

	match support().await {
		Ok(()) => (),
		Err(_) => {
			if force {
				panic!("type unsupported in this browser")
			} else {
				return Ok(());
			}
		}
	}

	let request = Flag::new();
	let response = Flag::new();

	let worker = WorkerBuilder::new()?
		.message_handler_async({
			let assert_received = assert_received.clone();
			let response = response.clone();

			move |_, event| {
				let assert_received = assert_received.clone();
				let response = response.clone();

				async move {
					let mut messages = event.messages().unwrap().into_iter();
					assert_eq!(messages.len(), 2);

					let value: T = messages.next().unwrap().serialize_as().unwrap();
					assert_received(&value).await;

					let value = messages.next().unwrap().serialize().unwrap();
					let value: T = value.try_into().unwrap();
					assert_received(&value).await;

					response.signal();
				}
			}
		})
		.spawn({
			let request = request.clone();
			move |context| {
				context.set_message_handler_async(move |context, event| {
					let assert_received = assert_received.clone();
					let context = context.clone();

					async move {
						let mut messages = event.messages().unwrap().into_iter();
						assert_eq!(messages.len(), 2);

						let value_1: T = messages.next().unwrap().serialize_as().unwrap();
						assert_received(&value_1).await;

						let value_2 = messages.next().unwrap().serialize().unwrap();
						let value_2: T = value_2.try_into().unwrap();
						assert_received(&value_2).await;

						let old_value = value_1.clone();
						context.transfer_messages([value_1, value_2]).unwrap();
						assert_sent(&old_value);
					}
				});

				request.signal();
			}
		});

	request.await;

	assert!(Message::from(init().await)
		.has_support()
		.into_inner()
		.unwrap()
		.is_ok());

	let value_1 = init().await;
	let value_2 = init().await;

	let old_value = value_1.clone();
	worker.transfer_messages([value_1, value_2]).unwrap();
	assert_sent(&old_value);

	response.await;

	worker.terminate();

	Ok(())
}

/// [`ArrayBuffer`].
#[wasm_bindgen_test]
async fn array_buffer() -> Result<()> {
	test_transfer(
		|| ready(Message::has_array_buffer_support()),
		true,
		|| async {
			let buffer = ArrayBuffer::new(1);
			let array = Uint8Array::new(&buffer);
			array.copy_from(&[42]);
			buffer
		},
		|buffer| assert_eq!(buffer.byte_length(), 0),
		|buffer| {
			let array = Uint8Array::new(buffer);
			assert_eq!(array.get_index(0), 42);

			async {}
		},
	)
	.await
}

/// [`AudioData`].
#[wasm_bindgen_test]
#[cfg(web_sys_unstable_apis)]
async fn audio_data() -> Result<()> {
	test_transfer(
		|| ready(Message::has_audio_data_support()),
		false,
		|| async {
			let init = AudioDataInit::new(
				&ArrayBuffer::new(42),
				AudioSampleFormat::U8,
				1,
				42,
				3000.,
				0.,
			);
			AudioData::new(&init).unwrap()
		},
		|data| assert_eq!(data.format(), None),
		|data| {
			let size = data.allocation_size(&AudioDataCopyToOptions::new(0));
			assert_eq!(size, 42);

			async {}
		},
	)
	.await
}

/// [`ImageBitmap`].
#[wasm_bindgen_test]
async fn image_bitmap() -> Result<()> {
	let _ = Message::has_image_bitmap_support().await;

	test_transfer(
		Message::has_image_bitmap_support,
		true,
		|| {
			let image = ImageData::new_with_sw(1, 1).unwrap();
			let promise = web_sys::window()
				.unwrap()
				.create_image_bitmap_with_image_data(&image)
				.unwrap();

			JsFuture::from(promise)
				.map(Result::unwrap)
				.map(ImageBitmap::unchecked_from_js)
		},
		|bitmap| {
			assert_eq!(bitmap.width(), 0);
			assert_eq!(bitmap.height(), 0);
		},
		|bitmap| {
			assert_eq!(bitmap.width(), 1);
			assert_eq!(bitmap.height(), 1);

			async {}
		},
	)
	.await
}

/// [`MessagePort`](web_sys::MessagePort).
#[wasm_bindgen_test]
async fn message_port() -> Result<()> {
	let flag = Flag::new();
	let closure: Closure<dyn Fn()> = Closure::new({
		let flag = flag.clone();
		move || flag.signal()
	});

	test_transfer(
		|| ready(Message::has_message_port_support()),
		true,
		|| async {
			let channel = MessageChannel::new().unwrap();

			channel
				.port2()
				.set_onmessage(Some(closure.as_ref().unchecked_ref()));
			channel.port1()
		},
		|_| (),
		{
			let flag = flag.clone();
			move |port: &MessagePort| {
				port.post_message(&JsValue::NULL).unwrap();

				let mut flag = flag.clone();
				async move {
					// The worker message handler will never respond back if the sender was not
					// properly transfered.
					let result = future::select(&mut flag, util::sleep(SIGNAL_DURATION)).await;
					assert!(matches!(result, Either::Left(((), _))));
					flag.reset();
				}
			}
		},
	)
	.await
}

/// [`OffscreenCanvas`].
#[wasm_bindgen_test]
async fn offscreen_canvas() -> Result<()> {
	test_transfer(
		|| ready(Message::has_offscreen_canvas_support()),
		false,
		|| async { OffscreenCanvas::new(1, 1).unwrap() },
		|canvas| {
			assert_eq!(canvas.width(), 0);
			assert_eq!(canvas.height(), 0);
		},
		|canvas| {
			assert_eq!(canvas.width(), 1);
			assert_eq!(canvas.height(), 1);

			async {}
		},
	)
	.await
}

/// [`ReadableStream`].
#[wasm_bindgen_test]
async fn readable_stream() -> Result<()> {
	test_transfer(
		|| ready(Message::has_readable_stream_support()),
		false,
		|| async { ReadableStream::new().unwrap() },
		|stream| assert!(stream.locked()),
		|stream| {
			assert!(!stream.locked());

			async {}
		},
	)
	.await
}

/// [`RtcDataChannel`](web_sys::RtcDataChannel).
#[wasm_bindgen_test]
async fn rtc_data_channel() -> Result<()> {
	test_transfer(
		|| ready(Message::has_rtc_data_channel_support()),
		false,
		|| async {
			let connection = RtcPeerConnection::new().unwrap();
			connection.create_data_channel("")
		},
		|channel| assert_eq!(channel.ready_state(), RtcDataChannelState::Closed),
		|channel| {
			assert_eq!(channel.ready_state(), RtcDataChannelState::Connecting);

			async {}
		},
	)
	.await
}

/// [`TransformStream`].
#[wasm_bindgen_test]
async fn transform_stream() -> Result<()> {
	test_transfer(
		|| ready(Message::has_transform_stream_support()),
		false,
		|| async { TransformStream::new().unwrap() },
		|stream| {
			assert!(stream.readable().locked());
			assert!(stream.writable().locked());
		},
		|stream| {
			assert!(!stream.readable().locked());
			assert!(!stream.writable().locked());

			async {}
		},
	)
	.await
}

/// [`VideoFrame`].
#[wasm_bindgen_test]
#[cfg(web_sys_unstable_apis)]
async fn video_frame() -> Result<()> {
	test_transfer(
		|| ready(Message::has_video_frame_support()),
		false,
		|| async {
			VideoFrame::new_with_u8_array_and_video_frame_buffer_init(
				&mut [0; 4],
				&VideoFrameBufferInit::new(1, 1, VideoPixelFormat::Rgba, 0.),
			)
			.unwrap()
		},
		|frame| {
			assert_eq!(frame.coded_width(), 0);
			assert_eq!(frame.coded_height(), 0);
			assert_eq!(frame.format(), None);
		},
		|frame| {
			assert_eq!(frame.coded_width(), 1);
			assert_eq!(frame.coded_height(), 1);
			assert_eq!(frame.format(), Some(VideoPixelFormat::Rgba));

			async {}
		},
	)
	.await
}

/// [`WritableStream`].
#[wasm_bindgen_test]
async fn writable_stream() -> Result<()> {
	test_transfer(
		|| ready(Message::has_writable_stream_support()),
		false,
		|| async { WritableStream::new().unwrap() },
		|stream| assert!(stream.locked()),
		|stream| {
			assert!(!stream.locked());

			async {}
		},
	)
	.await
}
