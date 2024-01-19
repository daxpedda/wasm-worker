//! Example solely use for debugging and testing.

fn main() {
	#[cfg(all(target_family = "wasm", target_os = "unknown"))]
	console_error_panic_hook::set_once();
	web_thread::spawn(|| ());
}
