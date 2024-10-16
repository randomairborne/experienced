#!/bin/sh
set -eu
echo "Checking Rust formatting..."
cargo +nightly fmt --check
echo "Checking web formatting..."
npm run prettier-check
echo "Checking build..."
cargo +nightly clippy -- -D warnings
echo "Running tests"
cargo +nightly test --all