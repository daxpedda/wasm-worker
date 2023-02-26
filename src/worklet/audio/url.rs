use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::future::FusedFuture;
use once_cell::sync::OnceCell;
use web_sys::{Blob, BlobPropertyBag, Url};

use crate::worklet::{WorkletModule, WorkletModuleError, WorkletModuleFuture};

static DEFAULT: OnceCell<AudioWorkletUrl> = OnceCell::new();

#[derive(Debug)]
pub struct AudioWorkletUrl(pub(super) String);

impl Drop for AudioWorkletUrl {
	fn drop(&mut self) {
		Url::revoke_object_url(&self.0).unwrap();
	}
}

impl AudioWorkletUrl {
	#[allow(clippy::should_implement_trait)]
	pub fn default() -> AudioWorkletUrlFuture {
		AudioWorkletUrlFuture(Some(WorkletModule::default()))
	}

	#[must_use]
	pub fn new(module: &WorkletModule) -> Self {
		let sequence = module.to_sequence(include_str!("worklet.js"));
		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property).unwrap();
		let url = Url::create_object_url_with_blob(&blob).unwrap();

		Self(url)
	}

	#[must_use]
	pub fn as_raw(&self) -> &str {
		&self.0
	}
}

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct AudioWorkletUrlFuture(Option<WorkletModuleFuture<'static, 'static, true>>);

impl AudioWorkletUrlFuture {
	#[track_caller]
	pub fn into_inner(&mut self) -> Option<Result<&'static AudioWorkletUrl, WorkletModuleError>> {
		let future = self.0.as_mut().expect("polled after `Ready`");

		if let Some(default) = DEFAULT.get() {
			if let Some(result) = self.0.take().unwrap().into_inner() {
				debug_assert!(result.is_ok());
			}

			return Some(Ok(default));
		}

		let result = future.into_inner()?;
		self.0.take();

		Some(match result {
			Ok(module) => Ok(DEFAULT.get_or_init(|| AudioWorkletUrl::new(module))),
			Err(error) => Err(error),
		})
	}
}

impl Future for AudioWorkletUrlFuture {
	type Output = Result<&'static AudioWorkletUrl, WorkletModuleError>;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let future = self.0.as_mut().expect("polled after `Ready`");

		if let Some(default) = DEFAULT.get() {
			self.0.take();

			return Poll::Ready(Ok(default));
		}

		let result = ready!(Pin::new(future).poll(cx));
		self.0.take();

		match result {
			Ok(module) => Poll::Ready(Ok(DEFAULT.get_or_init(|| AudioWorkletUrl::new(module)))),
			Err(error) => Poll::Ready(Err(error)),
		}
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for AudioWorkletUrlFuture {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}
