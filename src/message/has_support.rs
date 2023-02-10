use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

use super::{ImageBitmapSupportFuture, Message, SupportError};

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct HasSupportFuture(Option<Inner>);

#[derive(Debug)]
enum Inner {
	Ready(Result<(), SupportError>),
	ImageBitmap(ImageBitmapSupportFuture),
}

impl HasSupportFuture {
	pub(super) fn new(message: &Message) -> Self {
		match message {
			Message::ArrayBuffer(_) => {
				Self(Some(Inner::Ready(Message::has_array_buffer_support())))
			}
			#[cfg(web_sys_unstable_apis)]
			Message::AudioData(_) => Self(Some(Inner::Ready(Message::has_audio_data_support()))),
			Message::ImageBitmap(_) => Self(Some(Inner::ImageBitmap(
				Message::has_image_bitmap_support(),
			))),
			Message::MessagePort(_) => Self(Some(Inner::Ready(Err(SupportError::Undetermined)))),
			Message::OffscreenCanvas(_) => todo!(),
			Message::ReadableStream(_) => todo!(),
			Message::RtcDataChannel(_) => todo!(),
			Message::TransformStream(_) => todo!(),
			#[cfg(web_sys_unstable_apis)]
			Message::VideoFrame(_) => todo!(),
			Message::WritableStream(_) => todo!(),
		}
	}

	pub fn into_inner(&mut self) -> Option<Result<(), SupportError>> {
		match self.0.as_mut().expect("polled after `Ready`") {
			Inner::Ready(support) => {
				let support = *support;
				self.0.take();

				Some(support)
			}
			Inner::ImageBitmap(future) => future.into_inner(),
		}
	}
}

impl Future for HasSupportFuture {
	type Output = Result<(), SupportError>;

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
