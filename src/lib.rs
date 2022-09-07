use std::mem;

use js_sys::Array;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::Blob;
use web_sys::BlobPropertyBag;
use web_sys::Url;
use web_sys::Worker;

type WebWorkerContext = Box<dyn 'static + FnOnce() + Send>;

pub fn spawn<F: 'static + FnOnce() + Send>(f: F) {
    let script = include_str!("web_worker_closure.js");
    let header = format!("importScripts('{}');\n", wasm_bindgen::script_url());

    let sequence = Array::of2(&JsValue::from(header.as_str()), &JsValue::from(script));
    let mut property = BlobPropertyBag::new();
    property.type_("text/javascript");
    let blob = Blob::new_with_str_sequence_and_options(&sequence, &property).unwrap();

    let url = Url::create_object_url_with_blob(&blob).unwrap();

    let worker = Worker::new(&url).unwrap();

    let f: *mut WebWorkerContext = Box::into_raw(Box::new(Box::new(f)));

    let init = Array::of3(
        &wasm_bindgen::module(),
        &wasm_bindgen::memory(),
        &JsValue::from(f as usize),
    );

    if let Err(err) = worker.post_message(&init) {
        drop(unsafe { Box::from_raw(f) });
        Err(err).unwrap()
    }
}

pub fn spawn_fn(f: fn()) {
    let script = include_str!("web_worker_fn.js");
    let header = format!("importScripts('{}');\n", wasm_bindgen::script_url());

    let sequence = Array::of2(&JsValue::from(header.as_str()), &JsValue::from(script));
    let mut property = BlobPropertyBag::new();
    property.type_("text/javascript");
    let blob = Blob::new_with_str_sequence_and_options(&sequence, &property).unwrap();

    let url = Url::create_object_url_with_blob(&blob).unwrap();

    let worker = Worker::new(&url).unwrap();

    let init = Array::new();
    init.push(&wasm_bindgen::module());
    init.push(&wasm_bindgen::memory());
    init.push(&JsValue::from(f as usize));

    worker.post_message(&init).unwrap()
}

#[wasm_bindgen]
pub fn __wasm_thread_closure(ptr: usize) {
    let f = unsafe { Box::from_raw(ptr as *mut WebWorkerContext) };
    f();
}

#[wasm_bindgen]
pub fn __wasm_thread_fn(ptr: usize) {
    let f = unsafe { mem::transmute::<_, fn()>(ptr) };
    f();
}
