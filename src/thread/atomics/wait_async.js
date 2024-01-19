onmessage = event => {
    const [memory, index, value] = event.data
    Atomics.wait(new Int32Array(memory.buffer), index, value)
    postMessage(undefined)
}
