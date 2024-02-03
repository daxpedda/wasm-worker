globalThis.registerProcessor('__web_thread_worklet', class extends AudioWorkletProcessor {
    constructor(__web_thread_options) {
        super()

        const [__web_thread_module, __web_thread_memory, __web_thread_data] = __web_thread_options.processorOptions

        initSync(__web_thread_module, __web_thread_memory)
        __web_thread_worklet_entry(this, __web_thread_data)
    }

    process() {}
})
