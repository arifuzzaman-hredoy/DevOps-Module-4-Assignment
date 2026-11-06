#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR/backend"
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
