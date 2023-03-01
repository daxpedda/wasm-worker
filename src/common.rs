use std::borrow::Cow;
use std::sync::atomic::AtomicUsize;

use js_sys::WebAssembly::Global;
use js_sys::{Number, Object, Reflect};
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

const ERROR: &str = "expected wasm-bindgen `web` or `no-modules` target";

#[derive(Clone, Debug)]
pub enum ShimFormat<'global> {
	EsModule,
	Classic { global: Cow<'global, str> },
}

impl ShimFormat<'_> {
	pub(crate) fn default() -> Self {
		static SHIM_URL: Lazy<ShimFormat<'static>> =
			Lazy::new(|| match wasm_bindgen::shim_format() {
				Some(wasm_bindgen::ShimFormat::EsModule) => ShimFormat::EsModule,
				Some(wasm_bindgen::ShimFormat::NoModules { global_name }) => ShimFormat::Classic {
					global: global_name.into(),
				},
				Some(_) | None => unreachable!("{ERROR}"),
			});

		SHIM_URL.clone()
	}
}

pub(crate) static SHIM_URL: Lazy<String> = Lazy::new(|| wasm_bindgen::shim_url().expect(ERROR));

thread_local! {
	pub(crate) static EXPORTS: Lazy<Exports> = Lazy::new(|| wasm_bindgen::exports().unchecked_into());
}

#[wasm_bindgen]
extern "C" {
	#[allow(non_camel_case_types)]
	pub(crate) type Exports;

	#[wasm_bindgen(method, js_name = __wbindgen_thread_destroy)]
	pub(crate) unsafe fn thread_destroy(this: &Exports, tls_base: &Global, stack_alloc: &Global);

	#[wasm_bindgen(method, getter, js_name = __tls_base)]
	pub(crate) fn tls_base(this: &Exports) -> Global;

	#[wasm_bindgen(method, getter, js_name = __stack_alloc)]
	pub(crate) fn stack_alloc(this: &Exports) -> Global;
}

pub(crate) static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct Tls {
	pub(crate) id: usize,
	tls_base: f64,
	stack_alloc: f64,
}

impl Tls {
	thread_local! {
		static DESCRIPTOR: Lazy<Object> = Lazy::new(|| {
			let descriptor = Object::new();
			Reflect::set(&descriptor, &"value".into(), &"i32".into()).unwrap();
			descriptor
		});
	}

	pub(crate) fn new(id: usize, tls_base: &Global, stack_alloc: &Global) -> Self {
		let tls_base = Number::unchecked_from_js(tls_base.value()).value_of();
		let stack_alloc = Number::unchecked_from_js(stack_alloc.value()).value_of();

		Self {
			id,
			tls_base,
			stack_alloc,
		}
	}

	pub(crate) fn tls_base(&self) -> Global {
		Self::DESCRIPTOR.with(|descriptor| Global::new(descriptor, &self.tls_base.into()).unwrap())
	}

	pub(crate) fn stack_alloc(&self) -> Global {
		Self::DESCRIPTOR
			.with(|descriptor| Global::new(descriptor, &self.stack_alloc.into()).unwrap())
	}
}

pub(crate) static WAIT_ASYNC_SUPPORT: Lazy<bool> = Lazy::new(|| {
	#[wasm_bindgen]
	extern "C" {
		type Atomics;

		#[wasm_bindgen(static_method_of = Atomics, js_name = waitAsync, getter)]
		fn wait_async() -> JsValue;
	}

	!Atomics::wait_async().is_undefined()
});
