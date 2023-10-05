use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::FusedFuture;
use once_cell::sync::OnceCell;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{ImageBitmap, ImageData};

use super::super::MessageSupportError;
use crate::global::{Global, GlobalContext};

static SUPPORT: OnceCell<bool> = OnceCell::new();

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct ImageBitmapSupportFuture(Option<State>);

#[derive(Debug)]
enum State {
	Ready(bool),
	Create(JsFuture),
}

impl ImageBitmapSupportFuture {
	pub(in super::super) fn new() -> Result<Self, MessageSupportError> {
		if let Some(support) = SUPPORT.get() {
			Ok(Self(Some(State::Ready(*support))))
		} else {
			GlobalContext::with(|global| {
				match global {
					GlobalContext::Window(_) => (),
					GlobalContext::Worker(_) => {
						if !Global::has_worker() {
							return Err(MessageSupportError::Context);
						}
					}
					GlobalContext::Worklet => return Err(MessageSupportError::Context),
				}

				let image = ImageData::new_with_sw(1, 1).unwrap();

				let promise = match global {
					GlobalContext::Window(window) => {
						window.create_image_bitmap_with_image_data(&image)
					}
					GlobalContext::Worker(worker) => {
						worker.create_image_bitmap_with_image_data(&image)
					}
					GlobalContext::Worklet => unreachable!(),
				}
				.unwrap();

				Ok(Self(Some(State::Create(JsFuture::from(promise)))))
			})
		}
	}

	#[track_caller]
	#[allow(clippy::wrong_self_convention)]
	pub fn into_inner(&mut self) -> Option<bool> {
		let state = self.0.as_ref().expect("polled after `Ready`");

		if let Some(support) = SUPPORT.get() {
			if let State::Ready(new_support) = self.0.take().unwrap() {
				debug_assert_eq!(
					*support, new_support,
					"determining support has yielded different results"
				);
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
	type Output = bool;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let state = self.0.as_mut().expect("polled after `Ready`");

		if let Some(support) = SUPPORT.get() {
			if let State::Ready(new_support) = self.0.take().unwrap() {
				debug_assert_eq!(
					*support, new_support,
					"determining support has yielded different results"
				);
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
					debug_assert_eq!(
						support, *old_support,
						"determining support has yielded different results"
					);
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
