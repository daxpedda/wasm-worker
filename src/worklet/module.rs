use std::borrow::Cow;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::future::FusedFuture;
use js_sys::JsString;
use once_cell::sync::OnceCell;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{AbortController, RequestInit, Response};

use super::{ImportSupportFuture, ShimFormat};
use crate::common::SHIM_URL;
use crate::global::WindowOrWorker;

static DEFAULT_MODULE: OnceCell<WorkletModule> = OnceCell::new();

#[derive(Debug)]
pub struct WorkletModule(pub(super) Inner);

#[derive(Debug)]
pub(super) enum Inner {
	Import(String),
	Inline { shim: String, imports: String },
}

impl WorkletModule {
	#[allow(clippy::should_implement_trait)]
	pub fn default() -> WorkletModuleFuture<'static, 'static, true> {
		Self::new_internal(SHIM_URL.deref(), ShimFormat::default())
	}

	#[allow(clippy::new_ret_no_self)]
	pub fn new<'url, 'format, URL: Into<Cow<'url, str>>>(
		url: URL,
		format: ShimFormat<'format>,
	) -> WorkletModuleFuture<'url, 'format, false> {
		Self::new_internal(url, format)
	}

	fn new_internal<'url, 'format, const DEFAULT: bool, URL: Into<Cow<'url, str>>>(
		url: URL,
		format: ShimFormat<'format>,
	) -> WorkletModuleFuture<'url, 'format, DEFAULT> {
		let url = url.into();

		let inner = match format {
			ShimFormat::EsModule => {
				let mut import_support = super::has_import_support();

				if let Some(import_support) = import_support.into_inner() {
					if import_support {
						WorkletModuleFuture::<DEFAULT>::new_ready(&url)
					} else {
						WorkletModuleFuture::<DEFAULT>::new_fetch(&url, format)
					}
				} else {
					State::ImportSupport {
						url,
						future: import_support,
					}
				}
			}
			ShimFormat::Classic { .. } => WorkletModuleFuture::<DEFAULT>::new_fetch(&url, format),
		};

		WorkletModuleFuture(Some(inner))
	}

	fn new_inner(inner: Inner) -> Self {
		if let Inner::Inline { shim, .. } = &inner {
			wasm_bindgen::intern(shim);
		}

		Self(inner)
	}
}

impl Drop for WorkletModule {
	fn drop(&mut self) {
		if let Inner::Inline { shim, .. } = &self.0 {
			wasm_bindgen::unintern(shim);
		}
	}
}

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
		format: ShimFormat<'format>,
		abort: AbortController,
		future: JsFuture,
	},
	Text {
		format: ShimFormat<'format>,
		abort: AbortController,
		future: JsFuture,
	},
	Ready(WorkletModule),
}

impl WorkletModuleFuture<'_, '_, true> {
	#[track_caller]
	pub fn into_inner(&mut self) -> Option<Result<&'static WorkletModule, JsValue>> {
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
	pub fn into_inner(&mut self) -> Option<Result<WorkletModule, JsValue>> {
		Self::into_inner_internal(self).map(|result| {
			result.map(|module| {
				let CowModule::Owned(module) = module else { unreachable!()};
				module
			})
		})
	}
}

impl<const DEFAULT: bool> WorkletModuleFuture<'_, '_, DEFAULT> {
	#[track_caller]
	#[allow(clippy::wrong_self_convention)]
	fn into_inner_internal(&mut self) -> Option<Result<CowModule, JsValue>> {
		if DEFAULT {
			if let Some(default) = DEFAULT_MODULE.get() {
				self.0.take();

				return Some(Ok(default.into()));
			}
		}

		match self.0.as_mut().expect("polled after `Ready`") {
			State::ImportSupport { url, future } => {
				if let Some(import_support) = future.into_inner() {
					if import_support {
						let State::Ready(module) = WorkletModuleFuture::<DEFAULT>::new_ready(url) else {unreachable!()};
						self.0.take();

						Some(Ok(if DEFAULT {
							DEFAULT_MODULE.get_or_init(|| module).into()
						} else {
							module.into()
						}))
					} else {
						self.0 = Some(WorkletModuleFuture::<DEFAULT>::new_fetch(
							url,
							ShimFormat::EsModule,
						));
						None
					}
				} else {
					None
				}
			}
			State::Ready(_) => {
				let Some(State::Ready(module)) = self.0.take() else {unreachable!()};

				Some(Ok(if DEFAULT {
					DEFAULT_MODULE.get_or_init(|| module).into()
				} else {
					module.into()
				}))
			}
			_ => None,
		}
	}

	fn new_fetch<'url, 'format>(url: &str, format: ShimFormat<'format>) -> State<'url, 'format> {
		let abort = AbortController::new().unwrap();
		let mut init = RequestInit::new();
		init.signal(Some(&abort.signal()));

		let promise = WindowOrWorker::with(|global| {
			let global = global.expect("expected `Window` or `WorkerGlobalScope`");

			match global {
				WindowOrWorker::Window(window) => window.fetch_with_str_and_init(url, &init),
				WindowOrWorker::Worker(worker) => worker.fetch_with_str_and_init(url, &init),
			}
		});
		let future = JsFuture::from(promise);

		State::Fetch {
			format,
			abort,
			future,
		}
	}

	fn new_ready<'url, 'format>(url: &str) -> State<'url, 'format> {
		State::Ready(WorkletModule::new_inner(Inner::Import(format!(
			"import {{initSync, __wasm_worker_worklet_entry}} from '{url}';\n\n",
		))))
	}

	#[track_caller]
	fn poll_internal(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Result<CowModule, WorkletModuleError>> {
		if DEFAULT {
			if let Some(default) = DEFAULT_MODULE.get() {
				self.0.take();

				return Poll::Ready(Ok(default.into()));
			}
		}

		loop {
			match self.0.as_mut().expect("polled after `Ready`") {
				State::ImportSupport { url, future } => {
					let import_support = ready!(Pin::new(future).poll(cx));

					if import_support {
						self.0 = Some(Self::new_ready(url));
					} else {
						self.0 = Some(Self::new_fetch(url, ShimFormat::EsModule));
					}
				}
				State::Fetch { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					let Some(State::Fetch { format, abort, .. }) = self.0.take() else {unreachable!()};

					let response: Response = result.map_err(WorkletModuleError)?.unchecked_into();

					self.0 = Some(State::Text {
						format,
						abort,
						future: JsFuture::from(response.text().map_err(WorkletModuleError)?),
					});
				}
				State::Text { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					let Some(State::Text { format, .. }) = self.0.take() else {unreachable!()};

					let shim: JsString = result.map_err(WorkletModuleError)?.unchecked_into();

					let module = match format {
						ShimFormat::EsModule => {
							WorkletModule::new_inner(Inner::Import(shim.into()))
						}
						ShimFormat::Classic { global } => {
							#[rustfmt::skip]
							let imports = format!("\
                                const initSync = {global}.initSync;\n\
                                const __wasm_worker_worklet_entry = {global}.__wasm_worker_worklet_entry;\n\
                            ");
							WorkletModule::new_inner(Inner::Inline {
								shim: shim.into(),
								imports,
							})
						}
					};

					return Poll::Ready(Ok(if DEFAULT {
						DEFAULT_MODULE.get_or_init(|| module).into()
					} else {
						module.into()
					}));
				}
				State::Ready(_) => {
					let Some(State::Ready(module)) = self.0.take() else {unreachable!()};

					return Poll::Ready(Ok(if DEFAULT {
						DEFAULT_MODULE.get_or_init(|| module).into()
					} else {
						module.into()
					}));
				}
			}
		}
	}
}

impl<const DEFAULT: bool> Drop for WorkletModuleFuture<'_, '_, DEFAULT> {
	fn drop(&mut self) {
		if let Some(inner) = &self.0 {
			match inner {
				State::Fetch { abort, .. } | State::Text { abort, .. } => abort.abort(),
				_ => (),
			}
		}
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

#[derive(Debug)]
pub struct WorkletModuleError(JsValue);

impl Display for WorkletModuleError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "error fetching shim URL: {:?}", self.0)
	}
}

impl Error for WorkletModuleError {}
