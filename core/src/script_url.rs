use std::borrow::Cow;

use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::JsValue;
use web_sys::{console, Blob, BlobPropertyBag, Url};

#[must_use]
pub fn default_script_url() -> &'static ScriptUrl {
	static SCRIPT_URL: Lazy<ScriptUrl> =
		Lazy::new(|| ScriptUrl::new(&wasm_bindgen::script_url(), wasm_bindgen::shim_is_module()));

	&SCRIPT_URL
}

#[derive(Clone, Debug)]
pub struct ScriptUrl {
	url: String,
	is_module: bool,
}

impl From<ScriptUrl> for Cow<'_, ScriptUrl> {
	fn from(value: ScriptUrl) -> Self {
		Cow::Owned(value)
	}
}

impl<'url> From<&'url ScriptUrl> for Cow<'url, ScriptUrl> {
	fn from(value: &'url ScriptUrl) -> Self {
		Cow::Borrowed(value)
	}
}

impl ScriptUrl {
	#[must_use]
	pub fn new(url: &str, is_module: bool) -> Self {
		let script = if is_module {
			format!(
				"import wasm_bindgen from '{}';\n{}",
				url,
				include_str!("script.js")
			)
		} else {
			format!("importScripts('{}');\n{}", url, include_str!("script.js"))
		};

		let sequence = Array::of1(&JsValue::from(script));
		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property);

		let url = blob
			.and_then(|blob| Url::create_object_url_with_blob(&blob))
			.unwrap();

		Self { url, is_module }
	}

	#[must_use]
	pub fn url(&self) -> &str {
		&self.url
	}

	#[must_use]
	pub const fn is_module(&self) -> bool {
		self.is_module
	}
}

impl Drop for ScriptUrl {
	fn drop(&mut self) {
		if let Err(error) = Url::revoke_object_url(&self.url) {
			console::warn_1(&format!("worker `Url` could not be deallocated: {error:?}").into());
		}
	}
}
