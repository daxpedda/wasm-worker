use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::FusedFuture;
use once_cell::sync::OnceCell;
use wasm_bindgen_futures::JsFuture;
use web_sys::OfflineAudioContext;

static SUPPORT: OnceCell<bool> = OnceCell::new();

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct ImportSupportFuture(Option<State>);

#[derive(Debug)]
enum State {
	Ready(bool),
	Create(JsFuture),
}

impl ImportSupportFuture {
	pub(super) fn new() -> Self {
		if let Some(support) = SUPPORT.get() {
			Self(Some(State::Ready(*support)))
		} else {
			let context =
				OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(
					1, 1, 8000.,
				)
				.unwrap();
			let worklet = context.audio_worklet().unwrap();
			let promise = worklet
				.add_module("data:text/javascript,import'data:text/javascript,'")
				.unwrap();

			Self(Some(State::Create(JsFuture::from(promise))))
		}
	}

	#[track_caller]
	pub fn into_inner(&mut self) -> Option<bool> {
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

impl Future for ImportSupportFuture {
	type Output = bool;

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

				let support = result.is_ok();

				if let Err((old_support, _)) = SUPPORT.try_insert(support) {
					debug_assert_eq!(support, *old_support);
				}

				Poll::Ready(support)
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
