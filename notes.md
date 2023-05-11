# Documentation

`RUSTFLAGS="--cfg=web_sys_unstable_apis -Ctarget-feature=+atomics,+bulk-memory,+mutable-globals" cargo doc --all-features --no-deps`

# Run Examples

`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-server-runner cargo run --example testing --all-features`
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-server-runner WASM_SERVER_RUNNER_NO_MODULE=1 cargo run --example testing --all-features`

# ES Module Target Tests

`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis CHROMEDRIVER=chromedriver cargo test --all-features`
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis NO_HEADLESS=1 cargo test --all-features`

# Classic Target Tests

`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis CHROMEDRIVER=chromedriver WASM_BINDGEN_USE_NO_MODULE=1 cargo test --all-features`
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis GECKODRIVER=geckodriver WASM_BINDGEN_USE_NO_MODULE=1 cargo test --all-features`
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis GECKODRIVER=geckodriver cargo test --test no_module_support --all-features`
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis NO_HEADLESS=1 WASM_BINDGEN_USE_NO_MODULE=1 cargo test --all-features`
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis NO_HEADLESS=1 cargo test --test no_module_support --all-features`
