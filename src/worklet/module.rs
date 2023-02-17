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
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use wasm_bindgen_futures::JsFuture;
use web_sys::{AbortController, RequestInit, Response};

use super::{ImportSupportFuture, ShimFormat};
use crate::common::SHIM_URL;
use crate::global::WindowOrWorker;

static DEFAULT: OnceCell<WorkletModule> = OnceCell::new();

#[derive(Debug)]
pub struct WorkletModule {
	pub(super) shim: String,
	pub(super) imports: Option<String>,
}

impl WorkletModule {
	#[allow(clippy::should_implement_trait)]
	pub fn default() -> DefaultWorkletModuleFuture {
		DefaultWorkletModuleFuture(Some(Self::new(SHIM_URL.deref(), ShimFormat::default())))
	}

	#[allow(clippy::new_ret_no_self)]
	pub fn new<'url, 'format, URL: Into<Cow<'url, str>>>(
		url: URL,
		format: ShimFormat<'format>,
	) -> WorkletModuleFuture<'url, 'format> {
		let url = url.into();

		let inner = match format {
			ShimFormat::EsModule => {
				let mut import_support = super::has_import_support();

				if let Some(import_support) = import_support.into_inner() {
					if import_support {
						WorkletModuleFuture::new_ready(&url)
					} else {
						WorkletModuleFuture::new_fetch(&url, format)
					}
				} else {
					Inner::ImportSupport {
						url,
						future: import_support,
					}
				}
			}
			ShimFormat::Classic { .. } => WorkletModuleFuture::new_fetch(&url, format),
		};

		WorkletModuleFuture(Some(inner))
	}

	fn new_internal(shim: String, imports: Option<String>) -> Self {
		wasm_bindgen::intern(&shim);

		Self { shim, imports }
	}
}

impl Drop for WorkletModule {
	fn drop(&mut self) {
		wasm_bindgen::unintern(&self.shim);
	}
}

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct DefaultWorkletModuleFuture(Option<WorkletModuleFuture<'static, 'static>>);

impl DefaultWorkletModuleFuture {
	#[track_caller]
	pub fn into_inner(&mut self) -> Option<Result<&'static WorkletModule, JsValue>> {
		if let Some(default) = DEFAULT.get() {
			self.0.take();

			return Some(Ok(default));
		}

		if let Some(result) = self.0.as_mut().expect("polled after `Ready`").into_inner() {
			self.0.take();

			Some(match result {
				Ok(module) => Ok(DEFAULT.get_or_init(|| module)),
				Err(error) => Err(error),
			})
		} else {
			None
		}
	}
}

impl Future for DefaultWorkletModuleFuture {
	type Output = Result<&'static WorkletModule, WorkletModuleError>;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let inner = self.0.as_mut().expect("polled after `Ready`");

		if let Some(default) = DEFAULT.get() {
			self.0.take();

			return Poll::Ready(Ok(default));
		}

		let result = ready!(Pin::new(inner).poll(cx));
		self.0.take();

		match result {
			Ok(module) => Poll::Ready(Ok(DEFAULT.get_or_init(|| module))),
			Err(error) => Poll::Ready(Err(error)),
		}
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for DefaultWorkletModuleFuture {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct WorkletModuleFuture<'url, 'format>(Option<Inner<'url, 'format>>);

#[derive(Debug)]
enum Inner<'url, 'format> {
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

impl WorkletModuleFuture<'_, '_> {
	#[track_caller]
	pub fn into_inner(&mut self) -> Option<Result<WorkletModule, JsValue>> {
		match self.0.as_mut().expect("polled after `Ready`") {
			Inner::ImportSupport { url, future } => {
				if let Some(import_support) = future.into_inner() {
					if import_support {
						let Inner::Ready(module) = WorkletModuleFuture::new_ready(url) else {unreachable!()};
						self.0.take();

						Some(Ok(module))
					} else {
						self.0 = Some(WorkletModuleFuture::new_fetch(url, ShimFormat::EsModule));
						None
					}
				} else {
					None
				}
			}
			Inner::Ready(_) => {
				let Some(Inner::Ready(module)) = self.0.take() else {unreachable!()};

				Some(Ok(module))
			}
			_ => None,
		}
	}

	fn new_fetch<'url, 'format>(url: &str, format: ShimFormat<'format>) -> Inner<'url, 'format> {
		let abort = AbortController::new().unwrap_throw();
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

		Inner::Fetch {
			format,
			abort,
			future,
		}
	}

	fn new_ready<'url, 'format>(url: &str) -> Inner<'url, 'format> {
		Inner::Ready(WorkletModule::new_internal(
			format!("import {{initSync, __wasm_worker_worklet_entry}} from '{url}';\n\n",),
			None,
		))
	}
}

impl Drop for WorkletModuleFuture<'_, '_> {
	fn drop(&mut self) {
		if let Some(inner) = &self.0 {
			match inner {
				Inner::Fetch { abort, .. } | Inner::Text { abort, .. } => abort.abort(),
				_ => (),
			}
		}
	}
}

impl Future for WorkletModuleFuture<'_, '_> {
	type Output = Result<WorkletModule, WorkletModuleError>;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		loop {
			match self.0.as_mut().expect("polled after `Ready`") {
				Inner::ImportSupport { url, future } => {
					let import_support = ready!(Pin::new(future).poll(cx));

					if import_support {
						self.0 = Some(Self::new_ready(url));
					} else {
						self.0 = Some(Self::new_fetch(url, ShimFormat::EsModule));
					}
				}
				Inner::Fetch { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					self.0.take();

					let response: Response = result.map_err(WorkletModuleError)?.unchecked_into();

					let Some(Inner::Fetch { format, abort, .. }) = self.0.take() else {unreachable!()};
					self.0 = Some(Inner::Text {
						format,
						abort,
						future: JsFuture::from(response.text().map_err(WorkletModuleError)?),
					});
				}
				Inner::Text { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					let Some(Inner::Text { format, .. }) = self.0.take() else {unreachable!()};

					let shim: JsString = result.map_err(WorkletModuleError)?.unchecked_into();

					return Poll::Ready(Ok(match format {
						ShimFormat::EsModule => WorkletModule::new_internal(shim.into(), None),
						ShimFormat::Classic { global } => {
							#[rustfmt::skip]
							let imports = format!("\
                                const initSync = {global}.initSync;\n\
                                const __wasm_worker_dedicated_entry = {global}.__wasm_worker_dedicated_entry;\n\
                            ");
							WorkletModule::new_internal(shim.into(), Some(imports))
						}
					}));
				}
				Inner::Ready(_) => {
					let Some(Inner::Ready(module)) = self.0.take() else {unreachable!()};

					return Poll::Ready(Ok(module));
				}
			}
		}
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for WorkletModuleFuture<'_, '_> {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
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
