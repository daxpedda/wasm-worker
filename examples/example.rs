use std::panic;

use web_sys::console;

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    wasm_thread::spawn(|| console::log_1(&"a".into()));
    wasm_thread::spawn(|| console::log_1(&"b".into()));
    wasm_thread::spawn(|| console::log_1(&"c".into()));
    wasm_thread::spawn(|| console::log_1(&"d".into()));
    wasm_thread::spawn_fn(blubb);
    wasm_thread::spawn_fn(blubb);
    wasm_thread::spawn_fn(blubb);
    wasm_thread::spawn_fn(|| console::log_1(&"fake fn".into()));
    wasm_thread::spawn(|| wasm_thread::spawn(|| console::log_1(&"nested".into())));
}

fn blubb() {
    console::log_1(&"e".into())
}
