use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::future::FusedFuture;
use once_cell::sync::OnceCell;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{ImageBitmap, ImageData};

use super::super::MessageSupportError;
use crate::global::WindowOrWorker;

static SUPPORT: OnceCell<Result<(), MessageSupportError>> = OnceCell::new();

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct ImageBitmapSupportFuture(Option<State>);

#[derive(Debug)]
enum State {
	Ready(Result<(), MessageSupportError>),
	Create(JsFuture),
}

impl ImageBitmapSupportFuture {
	pub(in super::super) fn new() -> Self {
		if let Some(support) = SUPPORT.get() {
			Self(Some(State::Ready(*support)))
		} else {
			Self(Some(
				WindowOrWorker::with(|global| {
					let image = ImageData::new_with_sw(1, 1).unwrap();

					let promise = match global {
						WindowOrWorker::Window(window) => {
							window.create_image_bitmap_with_image_data(&image)
						}
						WindowOrWorker::Worker(worker) => {
							worker.create_image_bitmap_with_image_data(&image)
						}
					}
					.unwrap();

					State::Create(JsFuture::from(promise))
				})
				.unwrap_or(State::Ready(Err(MessageSupportError::Undetermined))),
			))
		}
	}

	#[track_caller]
	pub fn into_inner(&mut self) -> Option<Result<(), MessageSupportError>> {
		let state = self.0.as_ref().expect("polled after `Ready`");

		if let Some(support) = SUPPORT.get() {
			if let State::Ready(new_support) = self.0.take().unwrap() {
				debug_assert_eq!(*support, new_support);
			}

			return Some(*support);
		}

		if let State::Ready(support) = state {
			let support = *support;
			self.0.take();

			Some(support)
		} else {
			None
		}
	}
}

impl Future for ImageBitmapSupportFuture {
	type Output = Result<(), MessageSupportError>;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let state = self.0.as_mut().expect("polled after `Ready`");

		if let Some(support) = SUPPORT.get() {
			if let State::Ready(new_support) = self.0.take().unwrap() {
				debug_assert_eq!(*support, new_support);
			}

			return Poll::Ready(*support);
		}

		match state {
			State::Ready(support) => {
				let support = *support;
				self.0.take();

				Poll::Ready(support)
			}
			State::Create(future) => {
				let result = ready!(Pin::new(future).poll(cx));
				self.0.take();

				let bitmap: ImageBitmap = result.unwrap().unchecked_into();

				let support = super::test_support(&bitmap);

				if let Err((old_support, _)) = SUPPORT.try_insert(support) {
					debug_assert_eq!(support, *old_support);
				}

				Poll::Ready(support)
			}
		}
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for ImageBitmapSupportFuture {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}
