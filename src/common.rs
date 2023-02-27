use std::borrow::Cow;
use std::future::Future;
use std::ops::Deref;
use std::sync::atomic::AtomicUsize;

use js_sys::WebAssembly::Global;
use js_sys::{Function, Number, Object, Promise, Reflect};
use once_cell::sync::Lazy;
use wasm_bindgen::closure::Closure as JsClosure;
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

#[derive(Debug)]
pub(crate) enum Closure {
	Classic(JsClosure<dyn FnMut(web_sys::MessageEvent)>),
	Future(JsClosure<dyn FnMut(web_sys::MessageEvent) -> Promise>),
}

impl Deref for Closure {
	type Target = Function;

	fn deref(&self) -> &Self::Target {
		match self {
			Self::Classic(closure) => closure.as_ref(),
			Self::Future(closure) => closure.as_ref(),
		}
		.unchecked_ref()
	}
}

impl Closure {
	pub(crate) fn classic(closure: impl 'static + FnMut(web_sys::MessageEvent)) -> Self {
		Self::Classic(JsClosure::new(closure))
	}

	pub(crate) fn future<F: 'static + Future<Output = ()>>(
		mut closure: impl 'static + FnMut(web_sys::MessageEvent) -> F,
	) -> Self {
		let closure = JsClosure::new({
			move |event| {
				let closure = closure(event);
				wasm_bindgen_futures::future_to_promise(async move {
					closure.await;
					Ok(JsValue::UNDEFINED)
				})
			}
		});

		Self::Future(closure)
	}
}

thread_local! {
	pub(crate) static EXPORTS: Lazy<Exports> = Lazy::new(|| wasm_bindgen::exports().unchecked_into());
}

pub(crate) type Exports = __wasm_worker_Exports;

#[wasm_bindgen]
extern "C" {
	#[allow(non_camel_case_types)]
	pub(crate) type __wasm_worker_Exports;

	#[wasm_bindgen(method, js_name = __wbindgen_thread_destroy)]
	pub(crate) unsafe fn thread_destroy(
		this: &__wasm_worker_Exports,
		tls_base: &Global,
		stack_alloc: &Global,
	);

	#[wasm_bindgen(method, getter, js_name = __tls_base)]
	pub(crate) fn tls_base(this: &__wasm_worker_Exports) -> Global;

	#[wasm_bindgen(method, getter, js_name = __stack_alloc)]
	pub(crate) fn stack_alloc(this: &__wasm_worker_Exports) -> Global;
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
