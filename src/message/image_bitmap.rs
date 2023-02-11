use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_util::future::FusedFuture;
use once_cell::sync::OnceCell;
use once_cell::unsync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use wasm_bindgen_futures::JsFuture;
use web_sys::{ImageBitmap, ImageData, Window, WorkerGlobalScope};

use super::{util, SupportError};

static SUPPORT: OnceCell<Result<(), SupportError>> = OnceCell::new();

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct ImageBitmapSupportFuture(Option<Inner>);

#[derive(Debug)]
enum Inner {
	Ready(Result<(), SupportError>),
	Unknown,
	Create(JsFuture),
}

impl ImageBitmapSupportFuture {
	pub(super) fn new() -> Self {
		if let Some(support) = SUPPORT.get() {
			Self(Some(Inner::Ready(*support)))
		} else {
			Self(Some(Inner::Unknown))
		}
	}

	pub fn into_inner(&mut self) -> Option<Result<(), SupportError>> {
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

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		enum Global {
			Window(Window),
			Worker(WorkerGlobalScope),
		}

		thread_local! {
			static GLOBAL: Lazy<Option<Global>> = Lazy::new(|| {
				#[wasm_bindgen]
				extern "C" {
					type ImageBitmapGlobal;

					#[wasm_bindgen(method, getter, js_name = Window)]
					fn window(this: &ImageBitmapGlobal) -> JsValue;

					#[wasm_bindgen(method, getter, js_name = WorkerGlobalScope)]
					fn worker(this: &ImageBitmapGlobal) -> JsValue;
				}

				let global: ImageBitmapGlobal = js_sys::global().unchecked_into();

				if !global.window().is_undefined() {
					Some(Global::Window(global.unchecked_into()))
				} else if !global.worker().is_undefined() {
					Some(Global::Worker(global.unchecked_into()))
				} else {
					None
				}
			});
		}

		if let Some(support) = SUPPORT.get() {
			return Poll::Ready(*support);
		}

		let mut self_ = self.as_mut();

		loop {
			match self_.0.as_mut().expect("polled after `Ready`") {
				Inner::Ready(support) => {
					let support = *support;
					self.0.take();

					return Poll::Ready(support);
				}
				Inner::Unknown => {
					let promise = GLOBAL.with(|global| {
						if let Some(global) = global.deref() {
							let image = ImageData::new_with_sw(1, 1).unwrap_throw();

							match global {
								Global::Window(window) => {
									window.create_image_bitmap_with_image_data(&image)
								}
								Global::Worker(worker) => {
									worker.create_image_bitmap_with_image_data(&image)
								}
							}
							.ok()
						} else {
							None
						}
					});

					if let Some(promise) = promise {
						self_.0 = Some(Inner::Create(JsFuture::from(promise)));
					} else {
						self.0.take();
						return Poll::Ready(Err(SupportError::Undetermined));
					}
				}
				Inner::Create(future) => {
					let bitmap: ImageBitmap = ready!(Pin::new(future).poll(cx))
						.unwrap_throw()
						.unchecked_into();

					let support = util::has_support(&bitmap);

					self.0.take();
					SUPPORT.set(support).unwrap();
					return Poll::Ready(support);
				}
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
