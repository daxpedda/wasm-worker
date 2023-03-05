use std::cell::Cell;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::{Blob, BlobPropertyBag, Url, WorkerOptions};

use crate::common::{ShimFormat, SHIM_URL};

#[derive(Debug)]
pub struct WorkerUrl {
	pub(crate) url: String,
	is_module: bool,
}

impl Drop for WorkerUrl {
	fn drop(&mut self) {
		Url::revoke_object_url(&self.url).unwrap();
	}
}

impl WorkerUrl {
	#[allow(clippy::should_implement_trait)]
	pub fn default() -> Result<&'static Self, ModuleSupportError> {
		static WORKER_URL: Lazy<Option<WorkerUrl>> =
			Lazy::new(|| WorkerUrl::new(&SHIM_URL, &ShimFormat::default()).ok());

		WORKER_URL.deref().as_ref().ok_or(ModuleSupportError)
	}

	pub fn new(url: &str, format: &ShimFormat<'_>) -> Result<Self, ModuleSupportError> {
		let script = match &format {
			ShimFormat::EsModule => {
				if !Self::has_module_support() {
					return Err(ModuleSupportError);
				}

				format!(
					"import {{initSync, __wasm_worker_worker_entry}} from '{}';\n\n{}",
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
						const __wasm_worker_worker_entry = {global}.__wasm_worker_worker_entry;\n\
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
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property).unwrap();

		let url = Url::create_object_url_with_blob(&blob).unwrap();

		Ok(Self {
			url,
			is_module: matches!(format, ShimFormat::EsModule),
		})
	}

	#[must_use]
	pub const fn is_module(&self) -> bool {
		self.is_module
	}

	#[must_use]
	pub fn as_raw(&self) -> &str {
		&self.url
	}

	#[must_use]
	pub fn has_module_support() -> bool {
		static HAS_MODULE_SUPPORT: Lazy<bool> = Lazy::new(|| {
			#[wasm_bindgen]
			struct ModuleSupport(Rc<Cell<bool>>);

			#[wasm_bindgen]
			impl ModuleSupport {
				#[allow(unreachable_pub)]
				#[wasm_bindgen(getter = type)]
				pub fn type_(&self) {
					self.0.set(true);
				}
			}

			let tester = Rc::new(Cell::new(false));
			let worker_options =
				WorkerOptions::from(JsValue::from(ModuleSupport(Rc::clone(&tester))));
			let worker = web_sys::Worker::new_with_options("data:,", &worker_options).unwrap();
			worker.terminate();

			tester.get()
		});

		*HAS_MODULE_SUPPORT
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleSupportError;

impl Display for ModuleSupportError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "browser doesn't support worker modules")
	}
}

impl Error for ModuleSupportError {}
