Test with: `WASM_BINDGEN_EXTERNREF = "1"` `WASM_BINDGEN_WEAKREF = "1"`

`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-server-runner cargo run -p wasm-worker-core --example basic`
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-server-runner WASM_SERVER_RUNNER_NO_MODULE=1 cargo run -p wasm-worker-core --example basic`

`CHROMEDRIVER=chromedriver cargo test -p wasm-worker-core`
`NO_HEADLESS=1 cargo test -p wasm-worker-core`

`GECKODRIVER=geckodriver WASM_BINDGEN_USE_NO_MODULE=1 cargo test -p wasm-worker-core`
`GECKODRIVER=geckodriver cargo test -p wasm-worker-core --test no_module_support`
`NO_HEADLESS=1 WASM_BINDGEN_USE_NO_MODULE=1 cargo test -p wasm-worker-core`
