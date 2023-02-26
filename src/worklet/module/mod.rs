mod future;
mod polyfill;
mod support;

use std::borrow::Cow;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;

use js_sys::{Array, JsString};
use once_cell::sync::OnceCell;
use wasm_bindgen::JsValue;

pub use self::future::WorkletModuleFuture;
use self::polyfill::{PolyfillImport, PolyfillInline};
pub use self::support::ImportSupportFuture;
use crate::common::{ShimFormat, SHIM_URL};

static DEFAULT_MODULE: OnceCell<Option<WorkletModule>> = OnceCell::new();

#[derive(Debug)]
pub struct WorkletModule(Type);

#[derive(Debug)]
enum Type {
	Import(String),
	Inline { shim: String, imports: String },
}

impl WorkletModule {
	#[allow(clippy::should_implement_trait)]
	pub fn default() -> WorkletModuleFuture<'static, 'static, true> {
		Self::new_internal(SHIM_URL.deref(), ShimFormat::default())
	}

	#[allow(clippy::new_ret_no_self)]
	pub fn new<'url, 'format, URL>(
		url: URL,
		format: ShimFormat<'format>,
	) -> WorkletModuleFuture<'url, 'format, false>
	where
		URL: Into<Cow<'url, str>>,
	{
		Self::new_internal(url, format)
	}

	fn new_internal<'url, 'format, const DEFAULT: bool, URL: Into<Cow<'url, str>>>(
		url: URL,
		format: ShimFormat<'format>,
	) -> WorkletModuleFuture<'url, 'format, DEFAULT> {
		let url = url.into();

		match format {
			ShimFormat::EsModule => {
				let mut support = Self::has_import_support();

				match support.into_inner() {
					Some(true) => WorkletModuleFuture::new_ready(Type::import(&url)),
					Some(false) => WorkletModuleFuture::new_error(),
					None => WorkletModuleFuture::new_support(url, support),
				}
			}
			ShimFormat::Classic { global } => WorkletModuleFuture::new_fetch(&url, global),
		}
	}

	fn new_type(r#type: Type) -> Self {
		if let Type::Inline { shim, .. } = &r#type {
			wasm_bindgen::intern(shim);
		}

		Self(r#type)
	}

	pub(super) fn to_sequence(&self, worklet: &str) -> Array {
		match &self.0 {
			Type::Import(import) => Array::of3(
				&PolyfillImport::import().into(),
				&import.into(),
				&worklet.into(),
			),
			Type::Inline { shim, imports } => Array::of4(
				&PolyfillInline::script().into(),
				&shim.into(),
				&imports.into(),
				&worklet.into(),
			),
		}
	}

	pub fn has_import_support() -> ImportSupportFuture {
		ImportSupportFuture::new()
	}
}

impl Drop for WorkletModule {
	fn drop(&mut self) {
		if let Type::Inline { shim, .. } = &self.0 {
			wasm_bindgen::unintern(shim);
		}
	}
}

impl Type {
	fn import(url: &str) -> Self {
		Self::Import(format!(
			"import {{initSync, __wasm_worker_worklet_entry}} from '{url}';\n\n"
		))
	}

	fn inline(shim: JsString, global: &str) -> Self {
		#[rustfmt::skip]
		let imports = format!("\
			\nconst initSync = {global}.initSync;\n\
			const __wasm_worker_worklet_entry = {global}.__wasm_worker_worklet_entry;\n\n\
		");
		Self::Inline {
			shim: shim.into(),
			imports,
		}
	}
}

#[derive(Debug)]
pub enum WorkletModuleError {
	Support,
	Fetch(JsValue),
}

impl Display for WorkletModuleError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Support => write!(f, "browser doesn't support importing modules in worklets"),
			Self::Fetch(error) => write!(f, "error fetching shim URL: {error:?}"),
		}
	}
}

impl Error for WorkletModuleError {}
