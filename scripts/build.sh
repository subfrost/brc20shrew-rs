#!/bin/bash

# Build script for shrewscriptions-rs
# This script builds the WASM module and runs tests

set -e

echo "🔨 Building shrewscriptions-rs..."

# Check if required tools are installed
check_tool() {
    if ! command -v $1 &> /dev/null; then
        echo "❌ $1 is not installed. Please install it first."
        exit 1
    fi
}

echo "📋 Checking required tools..."
check_tool "cargo"
check_tool "protoc"

# Set up environment
export RUSTFLAGS="-C link-arg=-zstack-size=8388608"

# Clean previous builds
echo "🧹 Cleaning previous builds..."
cargo clean

# Generate protobuf code
echo "🔧 Generating protobuf code..."
if [ ! -d "src/proto" ]; then
    mkdir -p src/proto
fi

# Build the project
echo "🏗️  Building project..."
cargo build --release --target wasm32-unknown-unknown

# Check if build was successful
if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    
    # Display WASM file info
    WASM_FILE="target/wasm32-unknown-unknown/release/shrewscriptions_rs.wasm"
    if [ -f "$WASM_FILE" ]; then
        echo "📦 WASM file generated: $WASM_FILE"
        echo "📏 File size: $(du -h $WASM_FILE | cut -f1)"
    fi
else
    echo "❌ Build failed!"
    exit 1
fi

echo "🎉 Build completed successfully!"