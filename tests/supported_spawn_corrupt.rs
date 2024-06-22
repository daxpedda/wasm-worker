#![cfg(test)]
#![cfg(all(target_family = "wasm", target_feature = "atomics"))]

use futures_util::future;
use futures_util::future::Either;
use web_thread::Builder;

use crate::util;
use crate::util::{Flag, SIGNAL_DURATION};

#[wasm_bindgen_test::wasm_bindgen_test]
async fn builder_stack_size() {
	#[allow(clippy::large_stack_frames, clippy::missing_const_for_fn)]
	fn allocate_on_stack() {
		#[allow(clippy::large_stack_arrays, clippy::no_effect_underscore_binding)]
		let _test = [0_u8; 1024 * 1024 * 2];
	}

	let flag = Flag::new();
	Builder::new()
		.stack_size(1024 * 64)
		.spawn({
			let flag = flag.clone();
			move || {
				allocate_on_stack();
				flag.signal();
			}
		})
		.unwrap();

	assert!(matches!(
		future::select(flag, util::sleep(SIGNAL_DURATION)).await,
		Either::Right(_)
	));
}
