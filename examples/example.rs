use std::panic;

use wasm_thread::spawn;
use web_sys::console;

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    spawn(|| console::log_1(&"hello from worker".into()));
}
