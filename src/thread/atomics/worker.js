self.onmessage = __web_thread_event => {
	const [__web_thread_module, __web_thread_memory, __web_thread_data] = __web_thread_event.data

	initSync(__web_thread_module, __web_thread_memory)
	__web_thread_entry(__web_thread_data)
}
