//! Test each transferable type to be sent and received successfully. This is
//! important to assert that the support methods of [`Message`] are correct.

#![cfg(test)]
#![allow(clippy::missing_assert_message)]

mod util;

use std::any::{self, Any};
use std::fmt::Debug;
use std::future::{ready, Future};

use futures_util::future::{self, Either};
use futures_util::FutureExt;
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use web_thread::message::{Message, MessageSupportError};
use web_thread::WorkerBuilder;
use web_sys::{
	console, ImageBitmap, ImageData, MessageChannel, MessagePort, OffscreenCanvas, ReadableStream,
	RtcDataChannel, RtcDataChannelState, RtcPeerConnection, TransformStream, WritableStream,
};
#[cfg(web_sys_unstable_apis)]
use web_sys::{
	AudioData, AudioDataCopyToOptions, AudioDataInit, AudioSampleFormat, Response, VideoFrame,
	VideoFrameBufferInit, VideoPixelFormat, WebTransport, WebTransportBidirectionalStream,
	WebTransportReceiveStream, WebTransportSendStream,
};

use self::util::{Flag, SIGNAL_DURATION};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`RawMessage::serialize()`](web_thread::RawMessage::serialize).
#[wasm_bindgen_test]
async fn serialize() {
	Message::has_array_buffer_support().unwrap();

	let flag = Flag::new();

	let _worker = WorkerBuilder::new()
		.spawn_with_message(
			{
				let flag = flag.clone();
				move |context, messages| {
					let mut message = messages.into_iter().next().unwrap();

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
						message = message
							.serialize_as::<WebTransportReceiveStream>()
							.unwrap_err()
							.0;
						message = message
							.serialize_as::<WebTransportSendStream>()
							.unwrap_err()
							.0;
					}
					message.serialize_as::<WritableStream>().unwrap_err();

					flag.signal();
					context.close();
				}
			},
			[ArrayBuffer::new(1)],
		)
		.unwrap();

	flag.await;
}

/// Tests transferring `T`.
///
/// If `force` is `true` the test will fail if the type is not supported,
/// otherwise the test will be skipped.
async fn test_transfer<S, R, F1, F2, F3>(
	support: impl Fn() -> F1,
	force: bool,
	init: impl Fn() -> F2,
	assert_sent: impl 'static + Copy + Fn(&R) + Send,
	assert_received: impl 'static + Clone + Fn(&R) -> F3 + Send,
) where
	S: Any + Clone + AsRef<R> + JsCast,
	Message: From<S> + From<R>,
	R: Clone + JsCast + TryFrom<Message>,
	<R as TryFrom<Message>>::Error: Debug,
	F1: Future<Output = Result<bool, MessageSupportError>>,
	F2: Future<Output = S>,
	F3: Future<Output = ()>,
{
	if !support().await.unwrap() {
		if force {
			panic!("type unsupported in this browser")
		} else {
			let type_name = any::type_name::<S>().split("::").last().unwrap();
			console::error_1(&format!("`{type_name}` unsupported").into());
			return;
		}
	}

	let message = Message::from(JsValue::UNDEFINED.unchecked_into::<S>());
	match message.has_support() {
		Ok(mut future) => {
			assert!(future.into_inner().is_some());
		}
		Err(MessageSupportError::Context) => panic!(),
		Err(MessageSupportError::Undeterminable) => (),
	}

	let value_1 = init().await;
	let value_2 = init().await;
	let old_value = value_1.clone();

	let flag = Flag::new();

	let worker = WorkerBuilder::new()
		.message_handler_async({
			let assert_received = assert_received.clone();
			let flag = flag.clone();

			move |_, event| {
				let assert_received = assert_received.clone();
				let flag = flag.clone();

				async move {
					let mut messages = event.messages().unwrap().into_iter();
					assert_eq!(messages.len(), 2);

					let value: R = messages.next().unwrap().serialize_as().unwrap();
					assert_received(&value).await;

					let value = messages.next().unwrap().serialize().unwrap();
					let value: R = value.try_into().unwrap();
					assert_received(&value).await;

					flag.signal();
				}
			}
		})
		.spawn_async_with_message(
			move |context, messages| async move {
				let mut messages = messages.into_iter();
				assert_eq!(messages.len(), 2);

				let value_1: R = messages.next().unwrap().serialize_as().unwrap();
				assert_received(&value_1).await;

				let value_2 = messages.next().unwrap().serialize().unwrap();
				let value_2: R = value_2.try_into().unwrap();
				assert_received(&value_2).await;

				let old_value = value_1.clone();
				context.transfer_messages([value_1, value_2]).unwrap();
				assert_sent(&old_value);
			},
			[value_1, value_2],
		)
		.unwrap();

	assert_sent(old_value.as_ref());

	flag.await;

	worker.terminate();

	match Message::from(init().await).has_support() {
		Ok(mut future) => assert!(future.into_inner().unwrap()),
		Err(MessageSupportError::Context) => panic!(),
		Err(MessageSupportError::Undeterminable) => (),
	}
}

/// [`ArrayBuffer`].
#[wasm_bindgen_test]
async fn array_buffer() {
	test_transfer(
		|| ready(Message::has_array_buffer_support()),
		true,
		|| async {
			let buffer = ArrayBuffer::new(1);
			let array = Uint8Array::new(&buffer);
			array.copy_from(&[42]);
			buffer
		},
		|buffer: &ArrayBuffer| assert_eq!(buffer.byte_length(), 0),
		|buffer| {
			let array = Uint8Array::new(buffer);
			assert_eq!(array.get_index(0), 42);

			async {}
		},
	)
	.await;
}

/// [`AudioData`].
#[wasm_bindgen_test]
#[cfg(web_sys_unstable_apis)]
async fn audio_data() {
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
		|data: &AudioData| assert_eq!(data.format(), None),
		|data| {
			let size = data.allocation_size(&AudioDataCopyToOptions::new(0));
			assert_eq!(size, 42);

			async {}
		},
	)
	.await;
}

/// [`ImageBitmap`].
#[wasm_bindgen_test]
async fn image_bitmap() {
	let _: bool = Message::has_image_bitmap_support().unwrap().await;

	test_transfer(
		|| Message::has_image_bitmap_support().unwrap().map(Ok),
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
		|bitmap: &ImageBitmap| {
			assert_eq!(bitmap.width(), 0);
			assert_eq!(bitmap.height(), 0);
		},
		|bitmap| {
			assert_eq!(bitmap.width(), 1);
			assert_eq!(bitmap.height(), 1);

			async {}
		},
	)
	.await;
}

/// [`MessagePort`](web_sys::MessagePort).
#[wasm_bindgen_test]
async fn message_port() {
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
					// properly transferred.
					let result = future::select(&mut flag, util::sleep(SIGNAL_DURATION)).await;
					assert!(matches!(result, Either::Left(((), _))));
					flag.reset();
				}
			}
		},
	)
	.await;
}

/// [`OffscreenCanvas`].
#[wasm_bindgen_test]
async fn offscreen_canvas() {
	test_transfer(
		|| ready(Message::has_offscreen_canvas_support()),
		true,
		|| async { OffscreenCanvas::new(1, 1).unwrap() },
		|canvas: &OffscreenCanvas| {
			assert_eq!(canvas.width(), 0);
			assert_eq!(canvas.height(), 0);
		},
		|canvas| {
			assert_eq!(canvas.width(), 1);
			assert_eq!(canvas.height(), 1);

			async {}
		},
	)
	.await;
}

/// [`ReadableStream`].
#[wasm_bindgen_test]
async fn readable_stream() {
	#[cfg(not(web_sys_unstable_apis))]
	#[wasm_bindgen]
	extern "C" {
		#[wasm_bindgen(js_name = ReadableStream)]
		type ReadableStreamTest;

		#[wasm_bindgen(catch, constructor, js_class = "ReadableStream")]
		fn new_test() -> Result<ReadableStreamTest, JsValue>;
	}

	test_transfer(
		|| ready(Message::has_readable_stream_support()),
		false,
		#[cfg(web_sys_unstable_apis)]
		|| async { ReadableStream::new().unwrap() },
		#[cfg(not(web_sys_unstable_apis))]
		|| async {
			ReadableStreamTest::new_test()
				.unwrap()
				.unchecked_into::<ReadableStream>()
		},
		|stream: &ReadableStream| assert!(stream.locked()),
		|stream| {
			assert!(!stream.locked());

			async {}
		},
	)
	.await;
}

/// [`RtcDataChannel`](web_sys::RtcDataChannel).
#[wasm_bindgen_test]
async fn rtc_data_channel() {
	test_transfer(
		|| ready(Message::has_rtc_data_channel_support()),
		false,
		|| async {
			let connection = RtcPeerConnection::new().unwrap();
			connection.create_data_channel("")
		},
		|channel: &RtcDataChannel| assert_eq!(channel.ready_state(), RtcDataChannelState::Closed),
		|channel| {
			assert_eq!(channel.ready_state(), RtcDataChannelState::Connecting);

			async {}
		},
	)
	.await;
}

/// [`TransformStream`].
#[wasm_bindgen_test]
async fn transform_stream() {
	#[cfg(not(web_sys_unstable_apis))]
	#[wasm_bindgen]
	extern "C" {
		#[wasm_bindgen(js_name = TransformStream)]
		type TransformStreamTest;

		#[wasm_bindgen(catch, constructor, js_class = "TransformStream")]
		fn new_test() -> Result<TransformStreamTest, JsValue>;
	}

	test_transfer(
		|| ready(Message::has_transform_stream_support()),
		false,
		#[cfg(web_sys_unstable_apis)]
		|| async { TransformStream::new().unwrap() },
		#[cfg(not(web_sys_unstable_apis))]
		|| async {
			TransformStreamTest::new_test()
				.unwrap()
				.unchecked_into::<TransformStream>()
		},
		|stream: &TransformStream| {
			assert!(stream.readable().locked());
			assert!(stream.writable().locked());
		},
		|stream| {
			assert!(!stream.readable().locked());
			assert!(!stream.writable().locked());

			async {}
		},
	)
	.await;
}

/// [`VideoFrame`].
#[ignore = "Safari has a bug where sending multiple `VideoFrame`s will only send duplicates of the \
            first"]
#[wasm_bindgen_test]
#[cfg(web_sys_unstable_apis)]
async fn video_frame() {
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
		|frame: &VideoFrame| {
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
	.await;
}

#[cfg(web_sys_unstable_apis)]
async fn web_transport_echo_server_available() -> bool {
	let available = if let Ok(response) = JsFuture::from(
		web_sys::window()
			.unwrap()
			.fetch_with_str("https://echo.webtransport.day"),
	)
	.await
	{
		Response::unchecked_from_js(response).ok()
	} else {
		false
	};

	if !available {
		console::error_1(&"<https://echo.webtransport.day> is not available".into());
	}

	available
}

/// [`WebTransportReceiveStream`].
#[wasm_bindgen_test]
#[cfg(web_sys_unstable_apis)]
async fn web_transport_receive_stream() {
	test_transfer(
		|| async {
			#[wasm_bindgen]
			extern "C" {
				type WebTransportReceiveStreamTest;

				#[wasm_bindgen(method, getter, js_name = WebTransport)]
				fn has_web_transport(this: &WebTransportReceiveStreamTest) -> JsValue;
			}

			Ok(
				!WebTransportReceiveStreamTest::unchecked_from_js(js_sys::global().into())
					.has_web_transport()
					.is_undefined() && web_transport_echo_server_available().await,
			)
		},
		false,
		|| async {
			let transport = WebTransport::new("https://echo.webtransport.day").unwrap();
			JsFuture::from(transport.ready()).await.unwrap();
			let stream: WebTransportBidirectionalStream =
				JsFuture::from(transport.create_bidirectional_stream())
					.await
					.unwrap()
					.unchecked_into();
			stream.readable()
		},
		|stream: &ReadableStream| assert!(stream.locked()),
		|stream| {
			assert!(!stream.locked());

			async {}
		},
	)
	.await;
}

/// [`WebTransportSendStream`].
#[wasm_bindgen_test]
#[cfg(web_sys_unstable_apis)]
async fn web_transport_send_stream() {
	test_transfer(
		|| async {
			#[wasm_bindgen]
			extern "C" {
				type WebTransportSendStreamTest;

				#[wasm_bindgen(method, getter, js_name = WebTransport)]
				fn has_web_transport(this: &WebTransportSendStreamTest) -> JsValue;
			}

			Ok(
				!WebTransportSendStreamTest::unchecked_from_js(js_sys::global().into())
					.has_web_transport()
					.is_undefined() && web_transport_echo_server_available().await,
			)
		},
		false,
		|| async {
			let transport = WebTransport::new("https://echo.webtransport.day").unwrap();
			JsFuture::from(transport.ready()).await.unwrap();
			let stream: WebTransportBidirectionalStream =
				JsFuture::from(transport.create_bidirectional_stream())
					.await
					.unwrap()
					.unchecked_into();
			stream.writable()
		},
		|stream: &WritableStream| assert!(stream.locked()),
		|stream| {
			assert!(!stream.locked());

			async {}
		},
	)
	.await;
}

/// [`WritableStream`].
#[wasm_bindgen_test]
async fn writable_stream() {
	#[cfg(not(web_sys_unstable_apis))]
	#[wasm_bindgen]
	extern "C" {
		#[wasm_bindgen(js_name = WritableStream)]
		type WritableStreamTest;

		#[wasm_bindgen(catch, constructor, js_class = "WritableStream")]
		fn new_test() -> Result<WritableStreamTest, JsValue>;
	}

	test_transfer(
		|| ready(Message::has_writable_stream_support()),
		false,
		#[cfg(web_sys_unstable_apis)]
		|| async { WritableStream::new().unwrap() },
		#[cfg(not(web_sys_unstable_apis))]
		|| async {
			WritableStreamTest::new_test()
				.unwrap()
				.unchecked_into::<WritableStream>()
		},
		|stream: &WritableStream| assert!(stream.locked()),
		|stream| {
			assert!(!stream.locked());

			async {}
		},
	)
	.await;
}
