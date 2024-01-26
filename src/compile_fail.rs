//! Unfortunately this is the only known way to do compile-failure tests with
//! cross-compilation support.

#![allow(rustdoc::private_doc_tests)]

/// ```compile_fail,E0373
/// web_thread::scope(|scope| {
///     scope.spawn(|| {
///         let mut test = 0;
///         scope.spawn(|| test = 1);
///     });
/// });
/// ```
struct ScopeNested;

/// ```compile_fail,E0373
/// web_thread::web::scope_async(|scope| async {
///     scope.spawn(|| {
///         let mut test = 0;
///         scope.spawn(|| test = 1);
///     });
/// });
/// ```
#[cfg(target_family = "wasm")]
struct ScopeNestedAsync;

/// ```compile_fail,E0505
/// let mut test = String::new();
///
/// let future = web_thread::web::scope_async(|scope| async {
///     test.push_str("test");
/// });
///
/// drop(test);
/// ```
#[cfg(target_family = "wasm")]
struct ScopeAsyncAwait;

/// ```compile_fail,E0505
/// let mut test = String::new();
///
/// let future = web_thread::web::scope_async(|scope| async {
///     test.push_str("test");
/// }).into_wait();
///
/// drop(test);
/// ```
#[cfg(target_family = "wasm")]
struct ScopeAsyncIntoAwait;

/// ```compile_fail,E0505
/// async fn test() {
/// let mut test = String::new();
///
/// let future = web_thread::web::scope_async(|scope| async {
///     scope.spawn(|| test.push_str("test"));
/// }).into_wait().await;
///
/// drop(test);
/// }
/// ```
#[cfg(target_family = "wasm")]
struct ScopeNestedAsyncWait;
