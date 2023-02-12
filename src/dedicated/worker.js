self.onmessage = async __wasm_worker_event => {
	self.onmessage = undefined;

	const [__wasm_worker_module, __wasm_worker_memory, __wasm_worker_task] = __wasm_worker_event.data;

	await __wasm_worker_wasm_bindgen(__wasm_worker_module, __wasm_worker_memory);
	await Promise.resolve(__wasm_worker_entry(__wasm_worker_task));
};
