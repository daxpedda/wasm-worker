use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::FusedFuture;

use super::super::Message;
use super::{ImageBitmapSupportFuture, MessageSupportError};

impl Message {
	pub fn has_support(&self) -> Result<MessageSupportFuture, MessageSupportError> {
		MessageSupportFuture::new(self)
	}
}

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct MessageSupportFuture(Option<State>);

#[derive(Debug)]
enum State {
	Ready(bool),
	ImageBitmap(ImageBitmapSupportFuture),
}

impl MessageSupportFuture {
	fn new(message: &Message) -> Result<Self, MessageSupportError> {
		Ok(Self(Some(match message {
			Message::ArrayBuffer(_) => State::Ready(Message::has_array_buffer_support()?),
			#[cfg(web_sys_unstable_apis)]
			Message::AudioData(_) => State::Ready(Message::has_audio_data_support()?),
			Message::ImageBitmap(_) => State::ImageBitmap(Message::has_image_bitmap_support()?),
			Message::MessagePort(_) => State::Ready(Message::has_message_port_support()?),
			Message::OffscreenCanvas(_) => State::Ready(Message::has_offscreen_canvas_support()?),
			Message::ReadableStream(_) => State::Ready(Message::has_readable_stream_support()?),
			Message::RtcDataChannel(_) => State::Ready(Message::has_rtc_data_channel_support()?),
			Message::TransformStream(_) => State::Ready(Message::has_transform_stream_support()?),
			#[cfg(web_sys_unstable_apis)]
			Message::VideoFrame(_) => State::Ready(Message::has_video_frame_support()?),
			#[cfg(web_sys_unstable_apis)]
			Message::WebTransportReceiveStream(_) => {
				State::Ready(Message::has_web_transport_receive_stream_support()?)
			}
			#[cfg(web_sys_unstable_apis)]
			Message::WebTransportSendStream(_) => {
				State::Ready(Message::has_web_transport_send_stream_support()?)
			}
			Message::WritableStream(_) => State::Ready(Message::has_writable_stream_support()?),
		})))
	}

	#[track_caller]
	#[allow(clippy::wrong_self_convention)]
	pub fn into_inner(&mut self) -> Option<bool> {
		match self.0.as_mut().expect("polled after `Ready`") {
			State::Ready(support) => {
				let support = *support;
				self.0.take();

				Some(support)
			}
			State::ImageBitmap(future) => {
				let support = future.into_inner()?;
				self.0.take();

				Some(support)
			}
		}
	}
}

impl Future for MessageSupportFuture {
	type Output = bool;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		match self.0.as_mut().expect("polled after `Ready`") {
			State::Ready(support) => {
				let support = *support;
				self.0.take();

				Poll::Ready(support)
			}
			State::ImageBitmap(future) => {
				let support = ready!(Pin::new(future).poll(cx));
				self.0.take();

				Poll::Ready(support)
			}
		}
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for MessageSupportFuture {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}
