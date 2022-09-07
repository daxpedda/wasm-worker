self.onmessage = async event => {
    let [module, memory, work] = event.data;

    let wasm = await wasm_bindgen.initWithoutStart(module, memory);
    wasm.__wasm_thread_fn(work);
    wasm.__wbindgen_thread_destroy();
    self.close();
};
