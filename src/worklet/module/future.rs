use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::future::FusedFuture;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AbortController, RequestCache, RequestInit, Response};

use super::{ImportSupportFuture, Type, WorkletModule, WorkletModuleError, DEFAULT_MODULE};
use crate::global::WindowOrWorker;

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct WorkletModuleFuture<'url, 'format, const DEFAULT: bool>(Option<State<'url, 'format>>);

#[derive(Debug)]
enum State<'url, 'format> {
	ImportSupport {
		url: Cow<'url, str>,
		future: ImportSupportFuture,
	},
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
	Ready(Result<CowModule, WorkletModuleError>),
}

impl WorkletModuleFuture<'_, '_, true> {
	#[track_caller]
	pub fn into_inner(&mut self) -> Option<Result<&'static WorkletModule, WorkletModuleError>> {
		Self::into_inner_internal(self).map(|result| {
			result.map(|module| {
				let CowModule::Borrowed(module) = module else { unreachable!()};
				module
			})
		})
	}
}

impl WorkletModuleFuture<'_, '_, false> {
	#[track_caller]
	pub fn into_inner(&mut self) -> Option<Result<WorkletModule, WorkletModuleError>> {
		Self::into_inner_internal(self).map(|result| {
			result.map(|module| {
				let CowModule::Owned(module) = module else { unreachable!()};
				module
			})
		})
	}
}

impl<'url, 'format, const DEFAULT: bool> WorkletModuleFuture<'url, 'format, DEFAULT> {
	pub fn into_static(mut self) -> WorkletModuleFuture<'static, 'static, DEFAULT> {
		WorkletModuleFuture(match self.0.take() {
			Some(State::ImportSupport { url, future }) => Some(State::ImportSupport {
				url: Cow::Owned(url.into_owned()),
				future,
			}),
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
	fn into_inner_internal(&mut self) -> Option<Result<CowModule, WorkletModuleError>> {
		let state = self.0.as_mut().expect("polled after `Ready`");

		if DEFAULT {
			if let Some(default) = DEFAULT_MODULE.get() {
				if let Some(new_support) = self.abort() {
					debug_assert_eq!(default.is_some(), new_support);
				}

				return Some(
					default
						.as_ref()
						.map(CowModule::from)
						.ok_or(WorkletModuleError::Support),
				);
			}
		}

		match state {
			State::ImportSupport { url, future, .. } => {
				let support = future.into_inner()?;

				let State::Ready(Ok(module)) = State::ready::<DEFAULT>(Type::import(url)) else { unreachable!() };
				self.0.take();

				if support {
					Some(Ok(module))
				} else {
					Some(Err(WorkletModuleError::Support))
				}
			}
			State::Ready(_) => {
				let Some(State::Ready(module)) = self.0.take() else { unreachable!() };
				Some(module)
			}
			_ => None,
		}
	}

	#[track_caller]
	fn poll_internal(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Result<CowModule, WorkletModuleError>> {
		assert!(self.0.is_some(), "polled after `Ready`");

		if DEFAULT {
			if let Some(default) = DEFAULT_MODULE.get() {
				if let Some(new_support) = self.abort() {
					debug_assert_eq!(default.is_some(), new_support);
				}

				return Poll::Ready(
					default
						.as_ref()
						.map(CowModule::from)
						.ok_or(WorkletModuleError::Support),
				);
			}
		}

		loop {
			match self.0.as_mut().unwrap() {
				State::ImportSupport { url, future } => {
					let import_support = ready!(Pin::new(future).poll(cx));

					if import_support {
						self.0 = Some(State::ready::<DEFAULT>(Type::import(url)));
					} else {
						self.0 = Some(State::error::<DEFAULT>());
					}
				}
				State::Fetch { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					let Some(State::Fetch { global, abort, .. }) = self.0.take() else { unreachable!() };

					let response: Response =
						result.map_err(WorkletModuleError::Fetch)?.unchecked_into();
					let promise = response.text().map_err(WorkletModuleError::Fetch)?;

					self.0 = Some(State::Text {
						global,
						abort,
						future: JsFuture::from(promise),
					});
				}
				State::Text { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					let Some(State::Text { global, .. }) = self.0.take() else { unreachable!() };

					let shim = result.map_err(WorkletModuleError::Fetch)?.unchecked_into();

					self.0 = Some(State::ready::<DEFAULT>(Type::inline(shim, &global)));
				}
				State::Ready(_) => {
					let Some(State::Ready(module)) = self.0.take() else { unreachable!() };
					return Poll::Ready(module);
				}
			}
		}
	}

	pub(super) const fn new_support(url: Cow<'url, str>, future: ImportSupportFuture) -> Self {
		Self(Some(State::ImportSupport { url, future }))
	}

	pub(super) fn new_fetch(url: &str, global: Cow<'format, str>) -> Self {
		Self(Some(State::fetch(url, global)))
	}

	pub(super) fn new_ready(r#type: Type) -> Self {
		Self(Some(State::ready::<DEFAULT>(r#type)))
	}

	pub(super) fn new_error() -> Self {
		Self(Some(State::error::<DEFAULT>()))
	}

	fn abort(&mut self) -> Option<bool> {
		match self.0.take()? {
			State::ImportSupport { .. } => None,
			State::Fetch { abort, .. } | State::Text { abort, .. } => {
				abort.abort();
				None
			}
			State::Ready(support) => Some(support.is_ok()),
		}
	}
}

impl<const DEFAULT: bool> Drop for WorkletModuleFuture<'_, '_, DEFAULT> {
	fn drop(&mut self) {
		self.abort();
	}
}

impl Future for WorkletModuleFuture<'_, '_, true> {
	type Output = Result<&'static WorkletModule, WorkletModuleError>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Self::poll_internal(self, cx).map_ok(|module| {
			let CowModule::Borrowed(module) = module else { unreachable!()};
			module
		})
	}
}

impl Future for WorkletModuleFuture<'_, '_, false> {
	type Output = Result<WorkletModule, WorkletModuleError>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Self::poll_internal(self, cx).map_ok(|module| {
			let CowModule::Owned(module) = module else { unreachable!()};
			module
		})
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for WorkletModuleFuture<'_, '_, true> {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for WorkletModuleFuture<'_, '_, false> {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}

impl<'format> State<'_, 'format> {
	fn fetch(url: &str, global: Cow<'format, str>) -> Self {
		let abort = AbortController::new().unwrap();
		let mut init = RequestInit::new();
		init.signal(Some(&abort.signal()));
		init.cache(RequestCache::ForceCache);

		let promise = WindowOrWorker::with(|global| {
			let global = global.expect("expected `Window` or `WorkerGlobalScope`");

			match global {
				WindowOrWorker::Window(window) => window.fetch_with_str_and_init(url, &init),
				WindowOrWorker::Worker(worker) => worker.fetch_with_str_and_init(url, &init),
			}
		});
		let future = JsFuture::from(promise);

		State::Fetch {
			global,
			abort,
			future,
		}
	}

	fn ready<const DEFAULT: bool>(r#type: Type) -> Self {
		let module = WorkletModule::new_type(r#type);
		let module = if DEFAULT {
			DEFAULT_MODULE
				.get_or_init(|| Some(module))
				.as_ref()
				.unwrap()
				.into()
		} else {
			module.into()
		};

		State::Ready(Ok(module))
	}

	fn error<const DEFAULT: bool>() -> Self {
		if DEFAULT {
			if let Err((old_value, ..)) = DEFAULT_MODULE.try_insert(None) {
				debug_assert!(old_value.is_none());
			};
		}

		State::Ready(Err(WorkletModuleError::Support))
	}
}

#[derive(Debug)]
enum CowModule {
	Borrowed(&'static WorkletModule),
	Owned(WorkletModule),
}

impl From<&'static WorkletModule> for CowModule {
	fn from(value: &'static WorkletModule) -> Self {
		Self::Borrowed(value)
	}
}

impl From<WorkletModule> for CowModule {
	fn from(value: WorkletModule) -> Self {
		Self::Owned(value)
	}
}
