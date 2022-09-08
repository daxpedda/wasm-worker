use std::ops::Deref;

use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::Blob;
use web_sys::BlobPropertyBag;
use web_sys::Url;
use web_sys::Worker;

static URL: Lazy<WorkerUrl> = Lazy::new(worker_url);

enum WebWorkerContext {
    Closure(Box<dyn 'static + FnOnce() + Send>),
    Fn(fn()),
}

pub fn spawn<F: 'static + FnOnce() + Send>(f: F) {
    spawn_internal(WebWorkerContext::Closure(Box::new(f)));
}

pub fn spawn_fn(f: fn()) {
    spawn_internal(WebWorkerContext::Fn(f));
}

fn spawn_internal(context: WebWorkerContext) {
    let worker = Worker::new(&URL).unwrap();

    let context = Box::into_raw(Box::new(context));

    let init = Array::of3(
        &wasm_bindgen::module(),
        &wasm_bindgen::memory(),
        &(context as usize).into(),
    );

    if let Err(err) = worker.post_message(&init) {
        drop(unsafe { Box::from_raw(context) });
        Err(err).unwrap()
    }
}

#[wasm_bindgen]
pub fn __wasm_thread_entry(context: usize) {
    match *unsafe { Box::from_raw(context as *mut WebWorkerContext) } {
        WebWorkerContext::Closure(f) => f(),
        WebWorkerContext::Fn(f) => f(),
    }
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
