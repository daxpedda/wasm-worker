let wasm;

function __wasm_worker_try(f) {
	try {
		f();
	} catch (e) {
		return e;
	}
}

function __wasm_worker_close() {
	wasm.__wbindgen_thread_destroy();
	self.close();
}

self.onmessage = async event => {
	const [module, memory, task] = event.data;

	wasm = await wasm_bindgen(module, memory);

	if (await wasm.__wasm_worker_entry(task) === true) {
		__wasm_worker_close();
	}
};