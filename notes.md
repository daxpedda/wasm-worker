# Test
cargo test
CHROMEDRIVER=chromedriver cargo test --target wasm32-unknown-unknown
GECKODRIVER=geckodriver cargo test --target wasm32-unknown-unknown
CHROMEDRIVER=chromedriver RUSTFLAGS=-Ctarget-feature=+atomics,+bulk-memory cargo +nightly test --target wasm32-unknown-unknown -Zbuild-std=panic_abort,std
GECKODRIVER=geckodriver RUSTFLAGS=-Ctarget-feature=+atomics,+bulk-memory cargo +nightly test --target wasm32-unknown-unknown -Zbuild-std=panic_abort,std

# Lint
cargo clippy --all-targets
cargo clippy --all-targets --target wasm32-unknown-unknown
RUSTFLAGS=-Ctarget-feature=+atomics,+bulk-memory cargo +nightly clippy --all-targets --target wasm32-unknown-unknown -Zbuild-std=panic_abort,std

# Doc
cargo doc --no-deps --document-private-items
cargo doc --no-deps --document-private-items --target wasm32-unknown-unknown
RUSTDOCFLAGS=-Ctarget-feature=+atomics,+bulk-memory RUSTFLAGS=-Ctarget-feature=+atomics,+bulk-memory cargo +nightly doc --no-deps --document-private-items --target wasm32-unknown-unknown -Zbuild-std=panic_abort,std
