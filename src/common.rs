use once_cell::sync::Lazy;

#[derive(Clone, Copy, Debug)]
pub enum ShimFormat<'global> {
	EsModule,
	Classic { global: &'global str },
}

pub(crate) static SHIM_URL: Lazy<String> = Lazy::new(|| {
	wasm_bindgen::shim_url().expect("expected wasm-bindgen `web` or `no-modules` target")
});
