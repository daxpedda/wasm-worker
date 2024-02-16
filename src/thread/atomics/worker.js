self.onmessage = async __web_thread_event => {
	self.onmessage = undefined
	const [__web_thread_module, __web_thread_memory, __web_thread_data] = __web_thread_event.data

	const __web_thread_exports = initSync(__web_thread_module, __web_thread_memory)
	const __web_thread_terminate_index = await __web_thread_worker_entry(__web_thread_data)
	__web_thread_exports.__wbindgen_thread_destroy()
	const __web_thread_terminate_buffer = new Int32Array(__web_thread_memory.buffer)
	Atomics.store(__web_thread_terminate_buffer, __web_thread_terminate_index, 1)
	Atomics.notify(__web_thread_terminate_buffer, __web_thread_terminate_index)
	Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0)
}
