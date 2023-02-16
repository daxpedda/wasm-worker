use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_util::future::FusedFuture;

use super::super::Message;
use super::{ImageBitmapSupportFuture, SupportError};

impl Message {
	pub fn has_support(&self) -> MessageSupportFuture {
		MessageSupportFuture::new(self)
	}
}

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct MessageSupportFuture(Option<Inner>);

#[derive(Debug)]
enum Inner {
	Ready(Result<(), SupportError>),
	ImageBitmap(ImageBitmapSupportFuture),
}

impl MessageSupportFuture {
	fn new(message: &Message) -> Self {
		Self(Some(match message {
			Message::ArrayBuffer(_) => Inner::Ready(Message::has_array_buffer_support()),
			#[cfg(web_sys_unstable_apis)]
			Message::AudioData(_) => Inner::Ready(Message::has_audio_data_support()),
			Message::ImageBitmap(_) => Inner::ImageBitmap(Message::has_image_bitmap_support()),
			Message::MessagePort(_) => Inner::Ready(Message::has_message_port_support()),
			Message::OffscreenCanvas(_) => Inner::Ready(Message::has_offscreen_canvas_support()),
			Message::ReadableStream(_) => Inner::Ready(Message::has_readable_stream_support()),
			Message::RtcDataChannel(_) => Inner::Ready(Message::has_rtc_data_channel_support()),
			Message::TransformStream(_) => Inner::Ready(Message::has_transform_stream_support()),
			#[cfg(web_sys_unstable_apis)]
			Message::VideoFrame(_) => Inner::Ready(Message::has_video_frame_support()),
			Message::WritableStream(_) => Inner::Ready(Message::has_writable_stream_support()),
		}))
	}

	#[track_caller]
	pub fn into_inner(&mut self) -> Option<Result<(), SupportError>> {
		match self.0.as_mut().expect("polled after `Ready`") {
			Inner::Ready(support) => {
				let support = *support;
				self.0.take();

				Some(support)
			}
			Inner::ImageBitmap(future) => {
				let support = future.into_inner()?;
				self.0.take();

				Some(support)
			}
		}
	}
}

impl Future for MessageSupportFuture {
	type Output = Result<(), SupportError>;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let mut self_ = self.as_mut();

		match self_.0.as_mut().expect("polled after `Ready`") {
			Inner::Ready(support) => {
				let support = *support;
				self.0.take();

				Poll::Ready(support)
			}
			Inner::ImageBitmap(future) => {
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
