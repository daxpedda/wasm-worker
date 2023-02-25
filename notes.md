# Run Examples

`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-server-runner cargo run --example basic`
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-server-runner WASM_SERVER_RUNNER_NO_MODULE=1 cargo run --example basic`

# ES Module Target Tests

`CHROMEDRIVER=chromedriver cargo test`
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis CHROMEDRIVER=chromedriver cargo test`
`NO_HEADLESS=1 cargo test`

# Classic Target Tests

`CHROMEDRIVER=chromedriver WASM_BINDGEN_USE_NO_MODULE=1 cargo test`
`GECKODRIVER=geckodriver WASM_BINDGEN_USE_NO_MODULE=1 cargo test`
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis GECKODRIVER=geckodriver WASM_BINDGEN_USE_NO_MODULE=1 cargo test`
`GECKODRIVER=geckodriver cargo test --test no_module_support`
`NO_HEADLESS=1 WASM_BINDGEN_USE_NO_MODULE=1 cargo test`
`NO_HEADLESS=1 cargo test --test no_module_support`
