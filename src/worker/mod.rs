mod builder;
mod context;
mod handle;
mod support;
mod url;

use std::future::Future;

pub use self::builder::WorkerBuilder;
pub use self::context::WorkerContext;
pub use self::handle::Worker;
#[cfg(feature = "message")]
pub use self::handle::WorkerRef;
pub use self::support::{has_async_support, AsyncSupportError, AsyncSupportFuture};
use self::url::WORKER_URL;

/// # Examples
///
/// ```
/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
/// # #[wasm_bindgen_test::wasm_bindgen_test]
/// # fn test() {
/// wasm_worker::spawn(|_| ());
/// # }
/// ```
pub fn spawn<F>(task: F) -> Worker
where
	F: 'static + FnOnce(WorkerContext) + Send,
{
	WorkerBuilder::new().spawn(task)
}

/// # Examples
///
/// ```
/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
/// # #[wasm_bindgen_test::wasm_bindgen_test]
/// # fn test() {
/// wasm_worker::spawn_async(|_| async {});
/// # }
/// ```
pub fn spawn_async<F1, F2>(task: F1) -> Worker
where
	F1: 'static + FnOnce(WorkerContext) -> F2 + Send,
	F2: 'static + Future<Output = ()>,
{
	WorkerBuilder::new().spawn_async(task)
}
