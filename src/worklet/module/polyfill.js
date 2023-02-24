globalThis.TextDecoder = class TextDecoder {
    decode(arg) {
        if (typeof arg === 'undefined') {
            return '';
        } else {
            throw Error('TextDecoder stub called');
        }
    }
};

globalThis.TextEncoder = class TextEncoder {
    encode() {
        throw Error('TextEncoder stub called');
    }
};

