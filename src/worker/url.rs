use js_sys::Array;
use web_sys::{Blob, BlobPropertyBag, Url};

use crate::common::SHIM_URL;

thread_local! {
	pub(super) static WORKER_URL: WorkerUrl = WorkerUrl::new();
}

#[derive(Debug)]
pub(super) struct WorkerUrl(String);

impl Drop for WorkerUrl {
	fn drop(&mut self) {
		Url::revoke_object_url(&self.0).unwrap();
	}
}

impl WorkerUrl {
	fn new() -> Self {
		let script = format!(
			"import {{initSync, __web_thread_worker_entry}} from '{}';\n\n{}",
			*SHIM_URL,
			include_str!("worker.js")
		);

		let sequence = Array::of1(&script.into());
		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property).unwrap();

		let url = Url::create_object_url_with_blob(&blob).unwrap();

		Self(url)
	}

	#[must_use]
	pub(super) fn as_raw(&self) -> &str {
		&self.0
	}
}
