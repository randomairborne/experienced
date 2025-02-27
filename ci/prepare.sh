#!/bin/sh
cargo sqlx prepare --workspace -- --all-features --all-targets
