use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::{Blob, BlobPropertyBag, Url};

use super::ShimFormat;
use crate::common::SHIM_URL;

#[derive(Debug)]
pub struct WorkerUrl {
	pub(crate) url: String,
	is_module: bool,
}

impl Drop for WorkerUrl {
	fn drop(&mut self) {
		Url::revoke_object_url(&self.url).unwrap_throw();
	}
}

impl WorkerUrl {
	#[must_use]
	#[allow(clippy::should_implement_trait)]
	pub fn default() -> &'static Self {
		static WORKER_URL: Lazy<WorkerUrl> = Lazy::new(|| {
			const ERROR: &str = "expected wasm-bindgen `web` or `no-modules` target";
			WorkerUrl::new(
				&SHIM_URL,
				match &wasm_bindgen::shim_format() {
					Some(wasm_bindgen::ShimFormat::EsModule) => ShimFormat::EsModule,
					Some(wasm_bindgen::ShimFormat::NoModules { global_name }) => {
						ShimFormat::Classic {
							global: global_name,
						}
					}
					Some(_) | None => unreachable!("{ERROR}"),
				},
			)
		});

		&WORKER_URL
	}

	#[must_use]
	pub fn new(url: &str, format: ShimFormat<'_>) -> Self {
		let script = match format {
			ShimFormat::EsModule => {
				format!(
					"import {{initSync, __wasm_worker_entry}} from '{}';\n\n{}",
					url,
					include_str!("worker.js")
				)
			}
			ShimFormat::Classic { global } => {
				#[rustfmt::skip]
				let script = format!(
					"\
						importScripts('{}');\n\
						const initSync = {global}.initSync;\n\
						const __wasm_worker_entry = {global}.__wasm_worker_entry;\n\
						\n\
						{}\
					",
					url,
					include_str!("worker.js")
				);
				script
			}
		};

		let sequence = Array::of1(&script.into());
		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property);

		let url = blob
			.and_then(|blob| Url::create_object_url_with_blob(&blob))
			.unwrap_throw();

		Self {
			url,
			is_module: matches!(format, ShimFormat::EsModule),
		}
	}

	#[must_use]
	pub const fn is_module(&self) -> bool {
		self.is_module
	}

	#[must_use]
	pub fn as_raw(&self) -> &str {
		&self.url
	}
}
