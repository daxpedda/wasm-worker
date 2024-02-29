globalThis.__web_thread_register_processor = (__web_thread_name, __web_thread_processor) => {
    globalThis.registerProcessor(__web_thread_name, class extends AudioWorkletProcessor {
        constructor(__web_thread_options) {
            super()
            this.__web_thread_this = __web_thread_processor.instantiate(this, __web_thread_options)
        }

        process(__web_thread_inputs, __web_thread_outputs, __web_thread_parameters) {
            return this.__web_thread_this.process(__web_thread_inputs, __web_thread_outputs, __web_thread_parameters)
        }

        static get parameterDescriptors() {
            return __web_thread_processor.parameterDescriptors()
        }
    })
}

globalThis.registerProcessor('__web_thread_worklet', class extends AudioWorkletProcessor {
    constructor(__web_thread_options) {
        super()

        const [
            __web_thread_module,
            __web_thread_memory,
            __web_thread_worklet_lock,
            __web_thread_task,
        ] = __web_thread_options.processorOptions

        initSync(__web_thread_module, __web_thread_memory)
        const __web_thread_memory_array = new Int32Array(__web_thread_memory.buffer)
        Atomics.store(__web_thread_memory_array, __web_thread_worklet_lock, 0)
        Atomics.notify(__web_thread_memory_array, __web_thread_worklet_lock)

        __web_thread_worklet_entry(__web_thread_task)
    }

    process() { }
})
