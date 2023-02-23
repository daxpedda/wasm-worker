use js_sys::Array;
use once_cell::sync::Lazy;
use web_sys::{Blob, BlobPropertyBag, Url};

static POLYFILL_IMPORT: Lazy<PolyfillImport> = Lazy::new(|| {
	let sequence = Array::of1(&include_str!("polyfill.js").into());
	let mut property = BlobPropertyBag::new();
	property.type_("text/javascript");
	let blob = Blob::new_with_str_sequence_and_options(&sequence, &property).unwrap();

	let url = Url::create_object_url_with_blob(&blob).unwrap();
	let import = format!("import '{url}';\n");

	PolyfillImport { import, url }
});

pub(super) struct PolyfillImport {
	import: String,
	url: String,
}

impl PolyfillImport {
	pub(super) fn import() -> &'static str {
		&POLYFILL_IMPORT.import
	}
}

impl Drop for PolyfillImport {
	fn drop(&mut self) {
		Url::revoke_object_url(&self.url).unwrap();
	}
}

static POLYFILL_INLINE: Lazy<PolyfillInline> = Lazy::new(|| {
	let polyfill = include_str!("polyfill.js");
	wasm_bindgen::intern(polyfill);

	PolyfillInline(polyfill)
});

pub(super) struct PolyfillInline(&'static str);

impl PolyfillInline {
	pub(super) fn script() -> &'static str {
		POLYFILL_INLINE.0
	}
}

impl Drop for PolyfillInline {
	fn drop(&mut self) {
		wasm_bindgen::unintern(self.0);
	}
}
