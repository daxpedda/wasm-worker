#![allow(clippy::missing_docs_in_private_items, missing_docs)]

use web_sys::console;

fn main() {
	web_sys::console::log_1(&wasm_bindgen::exports());

	console::log_1(&"start".into());

	wasm_worker::spawn(|_| {
		console::log_1(&"thread".into());
	});
}
