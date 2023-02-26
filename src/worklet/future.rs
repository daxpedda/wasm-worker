use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::FusedFuture;
use js_sys::Array;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
	AudioWorkletGlobalScope, AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext,
};

use super::url::{WorkletUrl, WorkletUrlError, WorkletUrlFuture};
use super::Data;
use crate::common::ID_COUNTER;

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct WorkletFuture<'context>(Option<State<'context>>);

enum State<'context> {
	Url {
		context: Cow<'context, BaseAudioContext>,
		f: Box<dyn 'static + FnOnce(AudioWorkletGlobalScope, usize) + Send>,
		future: WorkletUrlFuture<'static, 'static, true>,
	},
	Add {
		context: Cow<'context, BaseAudioContext>,
		f: Box<dyn 'static + FnOnce(AudioWorkletGlobalScope, usize) + Send>,
		future: JsFuture,
	},
}

impl<'context> WorkletFuture<'context> {
	pub(super) fn new_url(
		context: Cow<'context, BaseAudioContext>,
		f: Box<dyn 'static + FnOnce(AudioWorkletGlobalScope, usize) + Send>,
		future: WorkletUrlFuture<'static, 'static, true>,
	) -> Self {
		Self(Some(State::Url { context, f, future }))
	}

	pub(super) fn new_add(
		context: Cow<'context, BaseAudioContext>,
		f: Box<dyn 'static + FnOnce(AudioWorkletGlobalScope, usize) + Send>,
		url: &WorkletUrl,
	) -> Self {
		Self(Some(State::new_add(context, f, url)))
	}

	pub fn into_static(self) -> WorkletFuture<'static> {
		WorkletFuture(match self.0 {
			Some(State::Url { context, f, future }) => Some(State::Url {
				context: Cow::Owned(context.into_owned()),
				f,
				future,
			}),
			Some(State::Add { context, f, future }) => Some(State::Add {
				context: Cow::Owned(context.into_owned()),
				f,
				future,
			}),
			None => None,
		})
	}
}

impl<'context> State<'context> {
	fn new_add(
		context: Cow<'context, BaseAudioContext>,
		f: Box<dyn 'static + FnOnce(AudioWorkletGlobalScope, usize) + Send>,
		url: &WorkletUrl,
	) -> Self {
		let promise = context.audio_worklet().unwrap().add_module(&url.0).unwrap();

		State::Add {
			context,
			f,
			future: JsFuture::from(promise),
		}
	}
}

impl Future for WorkletFuture<'_> {
	type Output = Result<(), WorkletUrlError>;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		loop {
			match self.0.as_mut().expect("polled after `Ready`") {
				State::Url { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					let Some(State::Url {context, f, ..}) = self.0.take() else { unreachable!() };

					let url = result?;

					self.0 = Some(State::new_add(context, f, url));
				}
				State::Add { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					let Some(State::Add { context, f, ..}) = self.0.take() else { unreachable!() };

					let result = result.unwrap();
					debug_assert!(result.is_undefined());

					let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
					let data = Box::into_raw(Box::new(Data { id, task: f }));

					let mut options = AudioWorkletNodeOptions::new();
					options.processor_options(Some(&Array::of3(
						&wasm_bindgen::module(),
						&wasm_bindgen::memory(),
						&data.into(),
					)));

					let _node = AudioWorkletNode::new_with_options(
						&context,
						"__wasm_worker_InitWasm",
						&options,
					)
					.unwrap();

					return Poll::Ready(Ok(()));
				}
			}
		}
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for WorkletFuture<'_> {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}

impl Debug for State<'_> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Url {
				context, future, ..
			} => formatter
				.debug_struct("Url")
				.field("context", context)
				.field("f", &"Box<FnOnce()>")
				.field("future", future)
				.finish(),
			Self::Add {
				context, future, ..
			} => formatter
				.debug_struct("Add")
				.field("context", context)
				.field("f", &"Box<FnOnce()>")
				.field("future", future)
				.finish(),
		}
	}
}
