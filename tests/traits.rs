use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomPinned;
use std::panic::{RefUnwindSafe, UnwindSafe};

use static_assertions::{assert_impl_all, assert_not_impl_any, assert_obj_safe};
#[cfg(target_family = "wasm")]
use wasm_bindgen_test::wasm_bindgen_test;
use web_thread::{Builder, JoinHandle, Scope, ScopedJoinHandle, Thread, ThreadId};

#[cfg(target_family = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
const fn basic() {
	assert_impl_all!(Builder: Debug, Send, Sync, Unpin, RefUnwindSafe, UnwindSafe);
	assert_not_impl_any!(Builder: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd);
	assert_obj_safe!(Builder);

	assert_impl_all!(JoinHandle<PhantomPinned>: Debug, Send, Sync, Unpin);
	assert_not_impl_any!(JoinHandle<PhantomPinned>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, RefUnwindSafe, UnwindSafe);
	assert_obj_safe!(JoinHandle<PhantomPinned>);

	assert_impl_all!(Scope<'_, '_>: Debug, Send, Sync, Unpin, RefUnwindSafe);
	assert_not_impl_any!(Scope<'_, '_>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, UnwindSafe);
	assert_obj_safe!(Scope<'_, '_>);

	assert_impl_all!(ScopedJoinHandle<'_, PhantomPinned>: Debug, Send, Sync, Unpin);
	assert_not_impl_any!(ScopedJoinHandle<'_, PhantomPinned>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, RefUnwindSafe, UnwindSafe);
	assert_obj_safe!(ScopedJoinHandle<'_, PhantomPinned>);

	assert_impl_all!(Thread: Clone, Debug, Send, Sync, Unpin, RefUnwindSafe, UnwindSafe);
	assert_not_impl_any!(Thread: Copy, Hash, Eq, PartialEq, Ord, PartialOrd);
	assert_obj_safe!(Thread);

	assert_impl_all!(ThreadId: Clone, Copy, Debug, Hash, Eq, PartialEq, Send, Sync, Unpin, RefUnwindSafe, UnwindSafe);
	assert_not_impl_any!(ThreadId: Ord, PartialOrd);
	assert_obj_safe!(ThreadId);
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
const fn web() {
	use web_thread::web::{JoinHandleFuture, ScopeFuture, ScopedJoinHandleFuture};

	assert_impl_all!(JoinHandleFuture<'_, PhantomPinned>: Debug, Send, Sync, Unpin);
	assert_not_impl_any!(JoinHandleFuture<'_, PhantomPinned>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, RefUnwindSafe, UnwindSafe);

	assert_impl_all!(ScopedJoinHandleFuture<'_, '_, PhantomPinned>: Debug, Send, Sync, Unpin);
	assert_not_impl_any!(ScopedJoinHandleFuture<'_, '_, PhantomPinned>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, RefUnwindSafe, UnwindSafe);

	assert_impl_all!(ScopeFuture<'_, '_, PhantomPinned, PhantomPinned>: Debug, Send, Sync, RefUnwindSafe);
	assert_impl_all!(ScopeFuture<'_, '_, (), PhantomPinned>: Unpin);
	assert_not_impl_any!(ScopeFuture<'_, '_, PhantomPinned, PhantomPinned>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, UnwindSafe);
}
