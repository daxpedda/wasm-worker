globalThis.__web_thread_register_processor = (name, processor) => {
    globalThis.registerProcessor(name, class extends AudioWorkletProcessor {
        constructor(options) {
            super()
            this.__web_thread_this = processor.instantiate(this, options)
        }

        process(inputs, outputs, parameters) {
            return this.__web_thread_this.process(inputs, outputs, parameters)
        }

        static get parameterDescriptors() {
            return processor.parameterDescriptors()
        }
    })
}

globalThis.registerProcessor('__web_thread_worklet', class extends AudioWorkletProcessor {
    constructor(options) {
        super()

        const [module, memory, workletLock, data] = options.processorOptions

        initSync(module, memory)
        const memoryArray = new Int32Array(memory.buffer)
        Atomics.store(memoryArray, workletLock, 0)
        Atomics.notify(memoryArray, workletLock)

        __web_thread_worklet_register(data)

        this.continueProcessing = true
        this.port.onmessage = event => {
            this.continueProcessing = false
            this.port.onmessage = undefined
            const [task, message] = event.data

            if (task === undefined)
                return

            __web_thread_worklet_entry(task, message, event.ports[0])
        }
    }

    process() {
        return this.continueProcessing
    }
})
