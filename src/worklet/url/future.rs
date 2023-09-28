use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::FusedFuture;
use js_sys::Array;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AbortController, RequestCache, RequestInit, Response};

use super::{WorkletUrl, WorkletUrlError, DEFAULT_URL};
use crate::common::ShimFormat;
use crate::global::GlobalContext;

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct WorkletUrlFuture<'format, const DEFAULT: bool>(Option<State<'format>>);

#[derive(Debug)]
enum State<'format> {
	Fetch {
		global: Cow<'format, str>,
		abort: AbortController,
		future: JsFuture,
	},
	Text {
		global: Cow<'format, str>,
		abort: AbortController,
		future: JsFuture,
	},
	Ready(Result<CowUrl, WorkletUrlError>),
}

impl WorkletUrlFuture<'_, true> {
	#[track_caller]
	pub fn into_inner(&mut self) -> Option<Result<&'static WorkletUrl, WorkletUrlError>> {
		Self::into_inner_internal(self).map(|result| {
			result.map(|url| {
				let CowUrl::Borrowed(url) = url else {
					unreachable!()
				};
				url
			})
		})
	}
}

impl WorkletUrlFuture<'_, false> {
	#[track_caller]
	pub fn into_inner(&mut self) -> Option<Result<WorkletUrl, WorkletUrlError>> {
		Self::into_inner_internal(self).map(|result| {
			result.map(|url| {
				let CowUrl::Owned(sequence) = url else {
					unreachable!()
				};
				WorkletUrl::new_internal(&sequence)
			})
		})
	}
}

impl<'format, const DEFAULT: bool> WorkletUrlFuture<'format, DEFAULT> {
	pub(super) fn new(url: &str, format: ShimFormat<'format>) -> Self {
		match format {
			ShimFormat::EsModule => Self::new_ready(WorkletUrl::new_import(url)),
			ShimFormat::Classic { global } => Self::new_fetch(url, global),
		}
	}

	pub fn into_static(mut self) -> WorkletUrlFuture<'static, DEFAULT> {
		WorkletUrlFuture(match self.0.take() {
			Some(State::Fetch {
				global,
				abort,
				future,
			}) => Some(State::Fetch {
				global: Cow::Owned(global.into_owned()),
				abort,
				future,
			}),
			Some(State::Text {
				global,
				abort,
				future,
			}) => Some(State::Text {
				global: Cow::Owned(global.into_owned()),
				abort,
				future,
			}),
			Some(State::Ready(result)) => Some(State::Ready(result)),
			None => None,
		})
	}

	#[track_caller]
	#[allow(clippy::wrong_self_convention)]
	fn into_inner_internal(&mut self) -> Option<Result<CowUrl, WorkletUrlError>> {
		let state = self.0.as_mut().expect("polled after `Ready`");

		if DEFAULT {
			if let Some(default) = DEFAULT_URL.get() {
				if let Some(new_support) = self.abort() {
					debug_assert_eq!(default.is_some(), new_support);
				}

				return Some(
					default
						.as_ref()
						.map(CowUrl::from)
						.ok_or(WorkletUrlError::Support),
				);
			}
		}

		if let State::Ready(_) = state {
			let Some(State::Ready(url)) = self.0.take() else {
				unreachable!()
			};
			Some(url)
		} else {
			None
		}
	}

	#[track_caller]
	fn poll_internal(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Result<CowUrl, WorkletUrlError>> {
		assert!(self.0.is_some(), "polled after `Ready`");

		if DEFAULT {
			if let Some(default) = DEFAULT_URL.get() {
				if let Some(new_support) = self.abort() {
					debug_assert_eq!(default.is_some(), new_support);
				}

				return Poll::Ready(
					default
						.as_ref()
						.map(CowUrl::from)
						.ok_or(WorkletUrlError::Support),
				);
			}
		}

		loop {
			match self.0.as_mut().unwrap() {
				State::Fetch { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					let Some(State::Fetch { global, abort, .. }) = self.0.take() else {
						unreachable!()
					};

					let response: Response =
						result.map_err(WorkletUrlError::Fetch)?.unchecked_into();
					let promise = response.text().map_err(WorkletUrlError::Fetch)?;

					self.0 = Some(State::Text {
						global,
						abort,
						future: JsFuture::from(promise),
					});
				}
				State::Text { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					let Some(State::Text { global, .. }) = self.0.take() else {
						unreachable!()
					};

					let shim = result.map_err(WorkletUrlError::Fetch)?.unchecked_into();

					self.0 = Some(State::ready::<DEFAULT>(WorkletUrl::new_inline(
						shim, &global,
					)));
				}
				State::Ready(_) => {
					let Some(State::Ready(url)) = self.0.take() else {
						unreachable!()
					};
					return Poll::Ready(url);
				}
			}
		}
	}

	pub(super) fn new_fetch(url: &str, global: Cow<'format, str>) -> Self {
		Self(Some(State::fetch(url, global)))
	}

	pub(super) fn new_ready(sequence: Array) -> Self {
		Self(Some(State::ready::<DEFAULT>(sequence)))
	}

	fn abort(&mut self) -> Option<bool> {
		match self.0.take()? {
			State::Fetch { abort, .. } | State::Text { abort, .. } => {
				abort.abort();
				None
			}
			State::Ready(support) => Some(support.is_ok()),
		}
	}
}

impl<const DEFAULT: bool> Drop for WorkletUrlFuture<'_, DEFAULT> {
	fn drop(&mut self) {
		self.abort();
	}
}

impl Future for WorkletUrlFuture<'_, true> {
	type Output = Result<&'static WorkletUrl, WorkletUrlError>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Self::poll_internal(self, cx).map_ok(|url| {
			let CowUrl::Borrowed(url) = url else {
				unreachable!()
			};
			url
		})
	}
}

impl Future for WorkletUrlFuture<'_, false> {
	type Output = Result<WorkletUrl, WorkletUrlError>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Self::poll_internal(self, cx).map_ok(|url| {
			let CowUrl::Owned(sequence) = url else {
				unreachable!()
			};
			WorkletUrl::new_internal(&sequence)
		})
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for WorkletUrlFuture<'_, true> {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for WorkletUrlFuture<'_, false> {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}

impl<'format> State<'format> {
	fn fetch(url: &str, global: Cow<'format, str>) -> Self {
		let abort = AbortController::new().unwrap();
		let mut init = RequestInit::new();
		init.signal(Some(&abort.signal()));
		init.cache(RequestCache::ForceCache);

		let promise = GlobalContext::with(|global| match global {
			GlobalContext::Window(window) => window.fetch_with_str_and_init(url, &init),
			GlobalContext::Worker(worker) => worker.fetch_with_str_and_init(url, &init),
			GlobalContext::Worklet => panic!("expected `Window` or `WorkerGlobalScope`"),
		});
		let future = JsFuture::from(promise);

		State::Fetch {
			global,
			abort,
			future,
		}
	}

	fn ready<const DEFAULT: bool>(sequence: Array) -> Self {
		let url = if DEFAULT {
			DEFAULT_URL
				.get_or_init(|| Some(WorkletUrl::new_internal(&sequence)))
				.as_ref()
				.unwrap()
				.into()
		} else {
			sequence.into()
		};

		State::Ready(Ok(url))
	}
}

#[derive(Debug)]
enum CowUrl {
	Borrowed(&'static WorkletUrl),
	Owned(Array),
}

impl From<&'static WorkletUrl> for CowUrl {
	fn from(value: &'static WorkletUrl) -> Self {
		Self::Borrowed(value)
	}
}

impl From<Array> for CowUrl {
	fn from(value: Array) -> Self {
		Self::Owned(value)
	}
}
