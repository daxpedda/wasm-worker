#![cfg(test)]
// FP: Should be fixed in Rust v1.77.
#![cfg_attr(
	not(target_family = "wasm"),
	allow(clippy::semicolon_if_nothing_returned)
)]

#[cfg(not(target_family = "wasm"))]
use std::time;

use time::{Duration, Instant};
use web_thread::{Builder, Scope};
#[cfg(target_family = "wasm")]
use {
	wasm_bindgen_test::wasm_bindgen_test,
	web_thread::web::{self, BuilderExt, JoinHandleExt, ScopeExt, ScopedJoinHandleExt},
	web_time as time,
};

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
async fn park() {
	let start = Instant::now();

	#[cfg_attr(not(target_family = "wasm"), allow(unused_mut))]
	let mut handle = web_thread::spawn(|| {
		web_thread::park();
		web_thread::park_timeout(Duration::from_secs(1));
		#[allow(deprecated)]
		web_thread::park_timeout_ms(1000);
	});

	handle.thread().unpark();

	#[cfg(not(target_family = "wasm"))]
	handle.join().unwrap();
	#[cfg(target_family = "wasm")]
	if web::has_wait_support() && cfg!(not(unsupported_spawn_then_wait)) {
		handle.join().unwrap();
	} else {
		handle.join_async().await.unwrap();
	}

	let elapsed = start.elapsed();
	assert!(elapsed.as_secs() >= 2, "time: {elapsed:?}");
}

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
async fn sleep() {
	let start = Instant::now();

	#[cfg_attr(not(target_family = "wasm"), allow(unused_mut))]
	let mut handle = web_thread::spawn(|| {
		web_thread::sleep(Duration::from_secs(1));
		#[allow(deprecated)]
		web_thread::sleep_ms(1000);
	});

	#[cfg(not(target_family = "wasm"))]
	handle.join().unwrap();
	#[cfg(target_family = "wasm")]
	if web::has_wait_support() && cfg!(not(unsupported_spawn_then_wait)) {
		handle.join().unwrap();
	} else {
		handle.join_async().await.unwrap();
	}

	let elapsed = start.elapsed();
	assert!(elapsed.as_secs() >= 2, "time: {elapsed:?}");
}

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
async fn spawn() {
	#[cfg_attr(not(target_family = "wasm"), allow(unused_mut))]
	let mut handle = web_thread::spawn(|| ());

	#[cfg(not(target_family = "wasm"))]
	handle.join().unwrap();
	#[cfg(target_family = "wasm")]
	if web::has_wait_support() && cfg!(not(unsupported_spawn_then_wait)) {
		handle.join().unwrap();
	} else {
		handle.join_async().await.unwrap();
	}
}

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
async fn scope() {
	let mut test = 0;

	let task = scope_task(|scope| {
		scope.spawn(|| test = 1);
	});

	#[cfg(not(target_family = "wasm"))]
	web_thread::scope(task);
	#[cfg(target_family = "wasm")]
	if web::has_wait_support() && cfg!(not(unsupported_spawn_then_wait)) {
		web_thread::scope(task);
	} else {
		web::scope_async(move |scope| async move {
			task(scope);
		})
		.await;
	}

	assert_eq!(test, 1);
}

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
async fn scope_builder() {
	let mut test = 0;

	let task = scope_task(|scope| {
		Builder::new().spawn_scoped(scope, || test = 1).unwrap();
	});

	#[cfg(not(target_family = "wasm"))]
	web_thread::scope(task);
	#[cfg(target_family = "wasm")]
	if web::has_wait_support() && cfg!(not(unsupported_spawn_then_wait)) {
		web_thread::scope(task);
	} else {
		web::scope_async(move |scope| async move {
			task(scope);
		})
		.await;
	}

	assert_eq!(test, 1);
}

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
async fn builder() {
	#[cfg_attr(not(target_family = "wasm"), allow(unused_mut))]
	let mut handle = Builder::new()
		.stack_size(0)
		.spawn(|| assert_eq!(web_thread::current().name(), None))
		.unwrap();

	#[cfg(not(target_family = "wasm"))]
	handle.join().unwrap();
	#[cfg(target_family = "wasm")]
	if web::has_wait_support() && cfg!(not(unsupported_spawn_then_wait)) {
		handle.join().unwrap();
	} else {
		handle.join_async().await.unwrap();
	}
}

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
async fn builder_name() {
	#[cfg_attr(not(target_family = "wasm"), allow(unused_mut))]
	let mut handle = Builder::new()
		.stack_size(0)
		.name(String::from("test"))
		.spawn(|| assert_eq!(web_thread::current().name(), Some("test")))
		.unwrap();

	#[cfg(not(target_family = "wasm"))]
	handle.join().unwrap();
	#[cfg(target_family = "wasm")]
	if web::has_wait_support() && cfg!(not(unsupported_spawn_then_wait)) {
		handle.join().unwrap();
	} else {
		handle.join_async().await.unwrap();
	}
}

#[cfg_attr(not(target_family = "wasm"), pollster::test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
async fn is_finished() {
	#[cfg_attr(not(target_family = "wasm"), allow(unused_mut))]
	let mut handle = web_thread::spawn(|| {
		web_thread::park();
	});

	assert!(!handle.is_finished());

	handle.thread().unpark();

	#[cfg(not(target_family = "wasm"))]
	handle.join().unwrap();
	#[cfg(target_family = "wasm")]
	{
		handle.join_async().await.unwrap();
		assert!(handle.is_finished());
	}
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
async fn join_async() {
	web_thread::spawn(|| ()).join_async().await.unwrap();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
fn has_thread_support() {
	assert!(web::has_spawn_support());
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
async fn spawn_async() {
	let mut handle = web::spawn_async(|| async { assert_eq!(web_thread::current().name(), None) });

	if web::has_wait_support() && cfg!(not(unsupported_spawn_then_wait)) {
		handle.join().unwrap();
	} else {
		handle.join_async().await.unwrap();
	}
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
async fn builder_async() {
	let mut handle = Builder::new()
		.stack_size(0)
		.spawn_async(|| async { assert_eq!(web_thread::current().name(), None) })
		.unwrap();

	if web::has_wait_support() && cfg!(not(unsupported_spawn_then_wait)) {
		handle.join().unwrap();
	} else {
		handle.join_async().await.unwrap();
	}
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
async fn scope_spawn_async() {
	let mut test = 0;

	let task = scope_task(|scope| {
		scope.spawn_async(|| async { test = 1 });
	});

	if web::has_wait_support() && cfg!(not(unsupported_spawn_then_wait)) {
		web_thread::scope(task);
	} else {
		web::scope_async(move |scope| async move {
			task(scope);
		})
		.await;
	}

	assert_eq!(test, 1);
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
async fn scope_builder_async() {
	let mut test = 0;

	let task = scope_task(|scope| {
		Builder::new()
			.spawn_scoped_async(scope, || async { test = 1 })
			.unwrap();
	});

	if web::has_wait_support() && cfg!(not(unsupported_spawn_then_wait)) {
		web_thread::scope(task);
	} else {
		web::scope_async(move |scope| async move {
			task(scope);
		})
		.await;
	}

	assert_eq!(test, 1);
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
async fn scope_async() {
	let mut test = 0;

	web::scope_async(|scope| async {
		scope.spawn(|| test = 1);
	})
	.await;

	assert_eq!(test, 1);
}

#[cfg(all(target_family = "wasm", not(unsupported_spawn_then_wait)))]
#[wasm_bindgen_test]
async fn scope_async_drop() {
	if !web::has_wait_support() {
		return;
	}

	let borrow = String::new();

	drop(web::scope_async(|scope| async {
		scope.spawn(|| &borrow);
	}));

	drop(borrow);
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
async fn scope_join_async() {
	let mut test = 0;

	web::scope_async(|scope| async {
		scope.spawn(|| test = 1).join_async().await.unwrap();
	})
	.await;

	assert_eq!(test, 1);
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
async fn scope_async_wait() {
	let mut test = 0;

	web::scope_async(|scope| async {
		scope.spawn(|| test = 1);
	})
	.into_wait()
	.await
	.await;

	assert_eq!(test, 1);
}

#[cfg(all(target_family = "wasm", not(unsupported_spawn_then_wait)))]
#[wasm_bindgen_test]
async fn scope_async_into_wait_drop() {
	if !web::has_wait_support() {
		return;
	}

	let borrow = String::new();

	drop(
		web::scope_async(|scope| async {
			scope.spawn(|| &borrow);
		})
		.into_wait(),
	);

	drop(borrow);
}

#[cfg(all(target_family = "wasm", not(unsupported_spawn_then_wait)))]
#[wasm_bindgen_test]
async fn scope_async_wait_drop() {
	if !web::has_wait_support() {
		return;
	}

	let mut test = 0;

	drop(
		web::scope_async(|scope| async {
			scope.spawn(|| test = 1);
		})
		.into_wait()
		.await,
	);

	assert_eq!(test, 1);
}

#[cfg(all(target_family = "wasm", not(unsupported_spawn_then_wait)))]
#[wasm_bindgen_test]
async fn scope_async_join() {
	if !web::has_wait_support() {
		return;
	}

	let mut test = 0;

	web::scope_async(|scope| async {
		scope.spawn(|| test = 1);
	})
	.into_wait()
	.await
	.join();

	assert_eq!(test, 1);
}

const fn scope_task<'env, F, T>(task: F) -> F
where
	F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T,
{
	task
}
