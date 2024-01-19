self.onmessage = __web_thread_event => {
	const [__web_thread_module, __web_thread_memory, __web_thread_data] = __web_thread_event.data

	initSync(__web_thread_module, __web_thread_memory)
	const __web_thread_terminate_index = __web_thread_entry(__web_thread_data)
	const __web_thread_terminate_buffer = new Int32Array(__web_thread_memory.buffer)
	__web_thread_terminate_buffer[__web_thread_terminate_index] = 1
	Atomics.notify(__web_thread_terminate_buffer, __web_thread_terminate_index)
	Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0)
}
