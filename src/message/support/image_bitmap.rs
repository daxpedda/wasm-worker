use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::future::FusedFuture;
use once_cell::sync::OnceCell;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use wasm_bindgen_futures::JsFuture;
use web_sys::{ImageBitmap, ImageData};

use super::super::SupportError;
use crate::global::WindowOrWorker;

static SUPPORT: OnceCell<Result<(), SupportError>> = OnceCell::new();

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct ImageBitmapSupportFuture(Option<Inner>);

#[derive(Debug)]
enum Inner {
	Ready(Result<(), SupportError>),
	Create(JsFuture),
}

impl ImageBitmapSupportFuture {
	pub(in super::super) fn new() -> Self {
		if let Some(support) = SUPPORT.get() {
			Self(Some(Inner::Ready(*support)))
		} else {
			let promise = WindowOrWorker::with(|global| {
				if let Some(global) = global {
					let image = ImageData::new_with_sw(1, 1).unwrap_throw();

					match global {
						WindowOrWorker::Window(window) => {
							window.create_image_bitmap_with_image_data(&image)
						}
						WindowOrWorker::Worker(worker) => {
							worker.create_image_bitmap_with_image_data(&image)
						}
					}
					.ok()
				} else {
					None
				}
			});

			if let Some(promise) = promise {
				Self(Some(Inner::Create(JsFuture::from(promise))))
			} else {
				Self(Some(Inner::Ready(Err(SupportError::Undetermined))))
			}
		}
	}

	#[track_caller]
	pub fn into_inner(&mut self) -> Option<Result<(), SupportError>> {
		if let Some(support) = SUPPORT.get() {
			self.0.take();

			return Some(*support);
		}

		if let Inner::Ready(support) = self.0.as_ref().expect("polled after `Ready`") {
			let support = *support;
			self.0.take();

			Some(support)
		} else {
			None
		}
	}
}

impl Future for ImageBitmapSupportFuture {
	type Output = Result<(), SupportError>;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		if let Some(support) = SUPPORT.get() {
			self.0.take();

			return Poll::Ready(*support);
		}

		match self.0.as_mut().expect("polled after `Ready`") {
			Inner::Ready(support) => {
				let support = *support;
				self.0.take();

				Poll::Ready(support)
			}
			Inner::Create(future) => {
				let result = ready!(Pin::new(future).poll(cx));
				self.0.take();

				let bitmap: ImageBitmap = result.unwrap_throw().unchecked_into();

				let support = super::test_support(&bitmap);

				SUPPORT.set(support).unwrap();
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
