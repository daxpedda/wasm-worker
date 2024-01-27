# Test

## Native
cargo test

## Single-Threaded
CHROMEDRIVER=chromedriver cargo test --target wasm32-unknown-unknown
GECKODRIVER=geckodriver RUSTFLAGS="--cfg=unsupported_service --cfg=unsupported_shared_wait" cargo test --target wasm32-unknown-unknown

## Single-Threaded Doc Tests
CHROMEDRIVER=chromedriver cargo +nightly test --doc --target wasm32-unknown-unknown -Zdoctest-xcompile
GECKODRIVER=geckodriver RUSTFLAGS="--cfg=unsupported_service --cfg=unsupported_shared_wait" cargo +nightly test --doc --target wasm32-unknown-unknown -Zdoctest-xcompile

## Single-Threaded without Cross-Origin Isolation

WASM_BINDGEN_TEST_NO_ORIGIN_ISOLATION=1 CHROMEDRIVER=chromedriver cargo test --target wasm32-unknown-unknown
WASM_BINDGEN_TEST_NO_ORIGIN_ISOLATION=1 GECKODRIVER=geckodriver RUSTFLAGS="--cfg=unsupported_service --cfg=unsupported_shared_wait" cargo test --target wasm32-unknown-unknown

## Single-Threaded Doc Tests without Cross-Origin Isolation

WASM_BINDGEN_TEST_NO_ORIGIN_ISOLATION=1 CHROMEDRIVER=chromedriver cargo +nightly test --doc --target wasm32-unknown-unknown -Zdoctest-xcompile
WASM_BINDGEN_TEST_NO_ORIGIN_ISOLATION=1 GECKODRIVER=geckodriver RUSTFLAGS="--cfg=unsupported_service --cfg=unsupported_shared_wait" cargo +nightly test --doc --target wasm32-unknown-unknown -Zdoctest-xcompile

## Multi-Threaded

CHROMEDRIVER=chromedriver RUSTFLAGS="--cfg=unsupported_spawn_then_wait -Ctarget-feature=+atomics,+bulk-memory" cargo +nightly test --target wasm32-unknown-unknown -Zdoctest-xcompile -Zbuild-std=panic_abort,std
GECKODRIVER=geckodriver RUSTFLAGS="--cfg=unsupported_service --cfg=unsupported_shared_wait -Ctarget-feature=+atomics,+bulk-memory" cargo +nightly test --target wasm32-unknown-unknown -Zdoctest-xcompile -Zbuild-std=panic_abort,std

## Multi-Threaded without Cross-Origin Isolation

WASM_BINDGEN_TEST_NO_ORIGIN_ISOLATION=1 CHROMEDRIVER=chromedriver RUSTFLAGS="--cfg=unsupported_spawn -Ctarget-feature=+atomics,+bulk-memory" cargo +nightly test --target wasm32-unknown-unknown -Zdoctest-xcompile -Zbuild-std=panic_abort,std
WASM_BINDGEN_TEST_NO_ORIGIN_ISOLATION=1 GECKODRIVER=geckodriver RUSTFLAGS="--cfg=unsupported_spawn --cfg=unsupported_service --cfg=unsupported_shared_wait -Ctarget-feature=+atomics,+bulk-memory" cargo +nightly test --target wasm32-unknown-unknown -Zdoctest-xcompile -Zbuild-std=panic_abort,std

# Lint
cargo clippy --all-targets
cargo clippy --all-targets --target wasm32-unknown-unknown
RUSTFLAGS=-Ctarget-feature=+atomics,+bulk-memory cargo +nightly clippy --all-targets --target wasm32-unknown-unknown -Zbuild-std=panic_abort,std

# Doc Lint
cargo doc --no-deps --document-private-items --lib --examples
cargo doc --no-deps --document-private-items --lib --examples --target wasm32-unknown-unknown
RUSTDOCFLAGS=-Ctarget-feature=+atomics,+bulk-memory RUSTFLAGS=-Ctarget-feature=+atomics,+bulk-memory cargo +nightly doc --no-deps --document-private-items --lib --examples --target wasm32-unknown-unknown -Zbuild-std=panic_abort,std

# docs.rs Lint
RUSTDOCFLAGS=--cfg=docsrs cargo +nightly doc --no-deps --document-private-items --lib --examples
RUSTDOCFLAGS=--cfg=docsrs cargo +nightly doc --no-deps --document-private-items --lib --examples --target wasm32-unknown-unknown
RUSTDOCFLAGS="--cfg=docsrs -Ctarget-feature=+atomics,+bulk-memory" RUSTFLAGS=-Ctarget-feature=+atomics,+bulk-memory cargo +nightly doc --no-deps --document-private-items --lib --examples --target wasm32-unknown-unknown -Zbuild-std=panic_abort,std
