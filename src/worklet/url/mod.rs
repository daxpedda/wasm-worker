mod future;

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;

use js_sys::{Array, JsString};
use once_cell::sync::OnceCell;
use wasm_bindgen::JsValue;
use web_sys::{Blob, BlobPropertyBag, Url};

pub use self::future::WorkletUrlFuture;
use crate::common::{ShimFormat, SHIM_URL};

static DEFAULT_URL: OnceCell<Option<WorkletUrl>> = OnceCell::new();

#[derive(Debug)]
pub struct WorkletUrl(pub(super) String);

impl WorkletUrl {
	#[allow(clippy::should_implement_trait)]
	pub fn default() -> WorkletUrlFuture<'static, true> {
		WorkletUrlFuture::new(SHIM_URL.deref(), ShimFormat::default())
	}

	#[allow(clippy::new_ret_no_self)]
	pub fn new<'format>(
		url: &str,
		format: ShimFormat<'format>,
	) -> WorkletUrlFuture<'format, false> {
		WorkletUrlFuture::new(url, format)
	}

	fn new_import(url: &str) -> Array {
		let import = format!("import {{initSync, __wasm_worker_worklet_entry}} from '{url}';\n\n");
		Array::of2(&import.into(), &include_str!("worklet.js").into())
	}

	fn new_inline(shim: JsString, global: &str) -> Array {
		#[rustfmt::skip]
		let imports = format!("\
			\nconst initSync = {global}.initSync;\n\
			const __wasm_worker_worklet_entry = {global}.__wasm_worker_worklet_entry;\n\n\
		");
		Array::of3(
			&shim.into(),
			&imports.into(),
			&include_str!("worklet.js").into(),
		)
	}

	fn new_internal(sequence: &Array) -> Self {
		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(sequence, &property).unwrap();
		let url = Url::create_object_url_with_blob(&blob).unwrap();

		Self(url)
	}

	#[must_use]
	pub fn as_raw(&self) -> &str {
		&self.0
	}
}

impl Drop for WorkletUrl {
	fn drop(&mut self) {
		Url::revoke_object_url(&self.0).unwrap();
	}
}

#[derive(Debug)]
pub enum WorkletUrlError {
	Support,
	Fetch(JsValue),
}

impl Display for WorkletUrlError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Support => write!(f, "browser doesn't support importing modules in worklets"),
			Self::Fetch(error) => write!(f, "error fetching shim URL: {error:?}"),
		}
	}
}

impl Error for WorkletUrlError {}
