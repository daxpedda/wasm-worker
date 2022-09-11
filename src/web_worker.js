let wasm;

function __wasm_worker_try(f) {
    try {
        f();
    } catch (e) {
        return e;
    }
}

self.onmessage = async event => {
    let [module, memory, work] = event.data;

    wasm = await wasm_bindgen.initWithoutStart(module, memory);
    await wasm.__wasm_worker_entry(work);
    wasm.__wbindgen_thread_destroy();
    self.close();
};
