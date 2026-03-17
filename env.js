// Shim for metashrew-println's __log WASM import
// Used by wasm-bindgen-test-runner to satisfy the env.__log import
module.exports = {
    __log: function(ptr) {
        // no-op in test mode
    }
};
