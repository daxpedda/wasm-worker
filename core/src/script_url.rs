use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::{JsValue, ShimFormat, UnwrapThrowExt};
use web_sys::{Blob, BlobPropertyBag, Url};

#[must_use]
pub fn default_script_url() -> &'static ScriptUrl {
	const ERROR: &str = "expected wasm-bindgen `web` or `no-modules` target";
	static SCRIPT_URL: Lazy<ScriptUrl> = Lazy::new(|| {
		ScriptUrl::new(
			&wasm_bindgen::shim_url().expect(ERROR),
			match &wasm_bindgen::shim_format() {
				Some(ShimFormat::EsModule) => ScriptFormat::EsModule,
				Some(ShimFormat::NoModules { global_name }) => ScriptFormat::Classic {
					global: global_name,
				},
				Some(_) | None => panic!("{ERROR}"),
			},
		)
	});

	&SCRIPT_URL
}

#[derive(Debug)]
pub struct ScriptUrl {
	pub(crate) url: String,
	is_module: bool,
}

impl Drop for ScriptUrl {
	fn drop(&mut self) {
		Url::revoke_object_url(&self.url).expect_throw("can't fail");
	}
}

impl ScriptUrl {
	#[must_use]
	pub fn new(url: &str, format: ScriptFormat<'_>) -> Self {
		let script = match format {
			ScriptFormat::EsModule => {
				format!(
					"import __wasm_worker_wasm_bindgen, {{__wasm_worker_entry}} from '{}';\n\n{}",
					url,
					include_str!("script.js")
				)
			}
			ScriptFormat::Classic { global } => {
				#[rustfmt::skip]
				let script = format!(
					"\
						importScripts('{}');\n\
						const __wasm_worker_wasm_bindgen = {global};\n\
						const __wasm_worker_entry = __wasm_worker_wasm_bindgen.__wasm_worker_entry;\n\
						\n\
						{}\
					",
					url,
					include_str!("script.js")
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
			.unwrap();

		Self {
			url,
			is_module: format.is_module(),
		}
	}

	#[must_use]
	pub const fn is_module(&self) -> bool {
		self.is_module
	}

	pub fn revoke(self) {
		Url::revoke_object_url(&self.url).expect_throw("can't fail");
	}
}

#[derive(Clone, Copy, Debug)]
pub enum ScriptFormat<'global> {
	EsModule,
	Classic { global: &'global str },
}

impl ScriptFormat<'_> {
	const fn is_module(self) -> bool {
		match self {
			ScriptFormat::EsModule => true,
			ScriptFormat::Classic { .. } => false,
		}
	}
}
