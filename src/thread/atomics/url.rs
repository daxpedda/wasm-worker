//! Worker script.

use js_sys::Array;
use web_sys::{Blob, BlobPropertyBag};

use crate::thread::js::META;

thread_local! {
	/// Object URL to the worker script.
	pub(super) static URL: Url = Url::new();
}

/// Wrapper around object URL to the worker script.
#[derive(Debug)]
pub(super) struct Url(String);

impl Drop for Url {
	fn drop(&mut self) {
		web_sys::Url::revoke_object_url(&self.0)
			.expect("`URL.revokeObjectURL()` should never throw");
	}
}

impl Url {
	/// Creates a new [`Url`].
	fn new() -> Self {
		let script = format!(
			"import {{initSync, __web_thread_entry}} from '{}';\n\n{}",
			META.url(),
			include_str!("worker.js")
		);

		let sequence = Array::of1(&script.into());
		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property)
			.expect("`new Blob()` should never throw");

		let url = web_sys::Url::create_object_url_with_blob(&blob)
			.expect("`URL.createObjectURL()` should never throw");

		Self(url)
	}

	/// Returns the object URL.
	#[must_use]
	pub(super) fn as_raw(&self) -> &str {
		&self.0
	}
}
