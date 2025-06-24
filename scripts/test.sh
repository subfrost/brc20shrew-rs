#!/bin/bash

# Test script for shrewscriptions-rs
# This script runs all tests including unit tests and integration tests

set -e

echo "🧪 Running tests for shrewscriptions-rs..."

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ cargo is not installed. Please install Rust first."
    exit 1
fi

# Run clippy for linting
echo "🔍 Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

# Run formatting check
echo "📝 Checking code formatting..."
cargo fmt --all -- --check

# Run unit tests
echo "🔬 Running unit tests..."
cargo test --lib

# Run integration tests
echo "🔗 Running integration tests..."
cargo test --test integration_tests

# Run view tests
echo "👁️  Running view tests..."
cargo test --test view_tests

# Run all tests with coverage if available
if command -v cargo-tarpaulin &> /dev/null; then
    echo "📊 Running tests with coverage..."
    cargo tarpaulin --out Html --output-dir coverage
    echo "📈 Coverage report generated in coverage/tarpaulin-report.html"
else
    echo "ℹ️  Install cargo-tarpaulin for coverage reports: cargo install cargo-tarpaulin"
fi

# Run benchmarks if available
if [ -d "benches" ]; then
    echo "⚡ Running benchmarks..."
    cargo bench
fi

echo "✅ All tests passed!"