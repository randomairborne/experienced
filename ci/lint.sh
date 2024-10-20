#!/bin/sh
set -eu
echo "Checking Rust formatting..."
cargo +nightly fmt --check --all
echo "Checking web formatting..."
npm run prettier-check
echo "Checking build..."
SQLX_OFFLINE=true cargo +nightly clippy --all -- -D warnings
echo "Running tests..."
cargo +nightly test --all