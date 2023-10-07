use js_sys::Array;
use web_sys::{Blob, BlobPropertyBag, Url};

use crate::common::SHIM_URL;

thread_local! {
	pub(super) static WORKLET_URL: WorkletUrl = WorkletUrl::new();
}

impl Drop for WorkletUrl {
	fn drop(&mut self) {
		Url::revoke_object_url(&self.0).unwrap();
	}
}

#[derive(Debug)]
pub(super) struct WorkletUrl(String);

impl WorkletUrl {
	fn new() -> Self {
		let import = format!(
			"import {{initSync, __wasm_worker_worklet_entry}} from '{}';\n\n",
			*SHIM_URL
		);

		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(
			&Array::of2(&import.into(), &include_str!("worklet.js").into()),
			&property,
		)
		.unwrap();
		let url = Url::create_object_url_with_blob(&blob).unwrap();

		Self(url)
	}

	#[must_use]
	pub(super) fn as_raw(&self) -> &str {
		&self.0
	}
}
