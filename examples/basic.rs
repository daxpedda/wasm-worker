#![allow(clippy::missing_docs_in_private_items, missing_docs)]

use wasm_worker::Close;
use web_sys::console;

fn main() {
	console::log_1(&"start".into());

	wasm_worker::spawn(|_| {
		console::log_1(&"thread".into());
		Close::Yes
	});
}
