#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets

# Guard: placeholder blake3 hash must never ship
if grep -q 'blake3:0000' component.manifest.json; then
    echo "ERROR: component.manifest.json still has placeholder blake3 hash"
    exit 1
fi
