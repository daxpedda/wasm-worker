use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::{JsValue, ShimFormat, UnwrapThrowExt};
use web_sys::{Blob, BlobPropertyBag, Url};

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
				&wasm_bindgen::shim_url().expect(ERROR),
				match &wasm_bindgen::shim_format() {
					Some(ShimFormat::EsModule) => WorkerUrlFormat::EsModule,
					Some(ShimFormat::NoModules { global_name }) => WorkerUrlFormat::Classic {
						global: global_name,
					},
					Some(_) | None => unreachable!("{ERROR}"),
				},
			)
		});

		&WORKER_URL
	}

	#[must_use]
	pub fn new(url: &str, format: WorkerUrlFormat<'_>) -> Self {
		let script = match format {
			WorkerUrlFormat::EsModule => {
				format!(
					"import {{initSync, __wasm_worker_entry}} from '{}';\n\n{}",
					url,
					include_str!("worker.js")
				)
			}
			WorkerUrlFormat::Classic { global } => {
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

		let sequence = Array::of1(&JsValue::from(script));
		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property);

		let url = blob
			.and_then(|blob| Url::create_object_url_with_blob(&blob))
			.unwrap_throw();

		Self {
			url,
			is_module: format.is_module(),
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

#[derive(Clone, Copy, Debug)]
pub enum WorkerUrlFormat<'global> {
	EsModule,
	Classic { global: &'global str },
}

impl WorkerUrlFormat<'_> {
	const fn is_module(self) -> bool {
		match self {
			WorkerUrlFormat::EsModule => true,
			WorkerUrlFormat::Classic { .. } => false,
		}
	}
}
