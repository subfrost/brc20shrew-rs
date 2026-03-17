#!/bin/bash
# Wrapper for wasm-bindgen-test-runner that provides the env.__log shim
# Sets NODE_PATH so Node.js can find our env.js module
export NODE_PATH="$(dirname "$0")/node_modules:${NODE_PATH}"
exec /home/ubuntu/.cache/.wasm-pack/wasm-bindgen-baa57e3e202e624b/wasm-bindgen-test-runner "$@"
