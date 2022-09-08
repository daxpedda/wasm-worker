use std::future::Future;
use std::ops::Deref;

use js_sys::Array;
use js_sys::Function;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::Blob;
use web_sys::BlobPropertyBag;
use web_sys::DedicatedWorkerGlobalScope;
use web_sys::MessageEvent;
use web_sys::Url;
use web_sys::Worker;

enum WorkerContext {
    Closure(Box<dyn 'static + FnOnce() + Send>),
    Fn(fn()),
    Future(Box<dyn 'static + Future<Output = ()> + Send + Unpin>),
}

pub fn spawn<F: 'static + FnOnce() + Send>(f: F) {
    spawn_internal(WorkerContext::Closure(Box::new(f)));
}

pub fn spawn_fn(f: fn()) {
    spawn_internal(WorkerContext::Fn(f));
}

pub fn spawn_async<F: 'static + Future<Output = ()> + Send + Unpin>(f: F) {
    spawn_internal(WorkerContext::Future(Box::new(f)));
}

fn spawn_internal(context: WorkerContext) {
    spawn_internal_ptr(Box::into_raw(Box::new(context)))
}

fn spawn_internal_ptr(context: *mut WorkerContext) {
    let result = GLOBAL.with(|global| match global {
        WindowOrWorker::Window => match WORKER_URL.with(|worker_url| Worker::new(worker_url)) {
            Ok(worker) => {
                NESTED_WORKER.with(|callback| worker.set_onmessage(Some(callback)));

                let init = Array::of3(
                    &wasm_bindgen::module(),
                    &wasm_bindgen::memory(),
                    &(context as usize).into(),
                );

                worker.post_message(&init)
            }
            Err(error) => Err(error),
        },
        WindowOrWorker::Worker(worker) => {
            worker.post_message(&f64::from_bits(context as u64).into())
        }
    });

    if let Err(err) = result {
        drop(unsafe { Box::from_raw(context) });
        Err(err).unwrap()
    }
}

#[wasm_bindgen]
pub async fn __wasm_thread_entry(context: usize) {
    match *unsafe { Box::from_raw(context as *mut WorkerContext) } {
        WorkerContext::Closure(f) => f(),
        WorkerContext::Fn(f) => f(),
        WorkerContext::Future(f) => f.await,
    }
}

thread_local! {
    static WORKER_URL: WorkerUrl = worker_url();
}

struct WorkerUrl(String);

impl Deref for WorkerUrl {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for WorkerUrl {
    fn drop(&mut self) {
        Url::revoke_object_url(&self.0).unwrap();
    }
}

fn worker_url() -> WorkerUrl {
    let header = format!("importScripts('{}');\n", wasm_bindgen::script_url());
    let script = include_str!("web_worker.js");

    let sequence = Array::of2(&JsValue::from(header.as_str()), &JsValue::from(script));
    let mut property = BlobPropertyBag::new();
    property.type_("text/javascript");
    let blob = Blob::new_with_str_sequence_and_options(&sequence, &property).unwrap();

    let url = Url::create_object_url_with_blob(&blob).unwrap();

    WorkerUrl(url)
}

thread_local! {
    static NESTED_WORKER: NestedWorker =
        NestedWorker(Closure::wrap(Box::new(|event: &MessageEvent| {
            let context = event.data().as_f64().unwrap().to_bits() as *mut WorkerContext;
            spawn_internal_ptr(context);
        })));
}

struct NestedWorker(Closure<dyn FnMut(&MessageEvent)>);

impl Deref for NestedWorker {
    type Target = Function;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unchecked_ref()
    }
}

thread_local! {
    static GLOBAL: WindowOrWorker = WindowOrWorker::new();
}

enum WindowOrWorker {
    Window,
    Worker(DedicatedWorkerGlobalScope),
}

impl WindowOrWorker {
    fn new() -> Self {
        #[wasm_bindgen]
        extern "C" {
            type Global;

            #[wasm_bindgen(method, getter, js_name = Window)]
            fn window(this: &Global) -> JsValue;

            #[wasm_bindgen(method, getter, js_name = DedicatedWorkerGlobalScope)]
            fn worker(this: &Global) -> JsValue;
        }

        let global: Global = js_sys::global().unchecked_into();

        if !global.window().is_undefined() {
            Self::Window
        } else if !global.worker().is_undefined() {
            Self::Worker(global.unchecked_into())
        } else {
            panic!("Only supported in a browser or web worker");
        }
    }
}
