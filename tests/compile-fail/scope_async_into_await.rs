//@only-target-wasm32-unknown-unknown

fn test() {
	let mut test = String::new();

	let future = web_thread::web::scope_async(|scope| async {
		test.push_str("test");
	})
	.into_wait();

	drop(test);
    //~^ ERROR: cannot move out of `test` because it is borrowed
}
