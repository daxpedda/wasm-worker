use std::panic;

use js_sys::Promise;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::console;
use web_sys::DedicatedWorkerGlobalScope;

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    for index in 0..10 {
        wasm_thread::spawn(move || console::log_1(&format!("Fn {index}").into()));
    }

    for index in 0..10 {
        wasm_thread::spawn_async(move || async move {
            console::log_1(&format!("Async {index}").into())
        });
    }

    wasm_thread::spawn_async(|| async {
        let mut index = 0;

        loop {
            console::log_1(&format!("Counter {index}").into());
            sleep(2000).await.unwrap();
            index += 1;
        }
    });
}

async fn sleep(ms: u32) -> Result<(), JsValue> {
    JsFuture::from(Promise::new(&mut |resolve, _reject| {
        let global: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
        global
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms.try_into().unwrap())
            .unwrap();
    }))
    .await
    .map(|_| ())
}
