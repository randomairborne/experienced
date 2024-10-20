#!/bin/sh
set -eu
echo "Checking Rust formatting..."
cargo +nightly fmt --check --all
echo "Checking web formatting..."
npm run prettier-check
echo "Checking build..."
cargo +nightly clippy --all -- -D warnings
echo "Running tests..."
cargo +nightly test --all