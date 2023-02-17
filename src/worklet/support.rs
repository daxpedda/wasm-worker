use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::future::FusedFuture;
use once_cell::sync::OnceCell;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::JsFuture;
use web_sys::OfflineAudioContext;

static SUPPORT: OnceCell<bool> = OnceCell::new();

pub fn has_import_support() -> ImportSupportFuture {
	if let Some(support) = SUPPORT.get() {
		ImportSupportFuture(Some(Inner::Ready(*support)))
	} else {
		ImportSupportFuture(Some(Inner::Unknown))
	}
}

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct ImportSupportFuture(Option<Inner>);

#[derive(Debug)]
enum Inner {
	Ready(bool),
	Unknown,
	Create(JsFuture),
}

impl ImportSupportFuture {
	#[track_caller]
	pub fn into_inner(&mut self) -> Option<bool> {
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

impl Future for ImportSupportFuture {
	type Output = bool;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		if let Some(support) = SUPPORT.get() {
			self.0.take();

			return Poll::Ready(*support);
		}

		loop {
			match self.0.as_mut().expect("polled after `Ready`") {
				Inner::Ready(support) => {
					let support = *support;
					self.0.take();

					return Poll::Ready(support);
				}
				Inner::Unknown => {
					let context = OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.).unwrap_throw();
					let worklet = context.audio_worklet().unwrap_throw();
					let promise = worklet
						.add_module("data:text/javascript,import'data:text/javascript,'")
						.unwrap_throw();

					self.0 = Some(Inner::Create(JsFuture::from(promise)));
				}
				Inner::Create(future) => {
					let result = ready!(Pin::new(future).poll(cx));
					let support = result.is_ok();

					self.0.take();
					SUPPORT.set(support).unwrap();
					return Poll::Ready(support);
				}
			}
		}
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for ImportSupportFuture {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}
