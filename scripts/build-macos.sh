#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
npm ci
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build

echo "Terminé. L'application se trouve dans src-tauri/target/release/bundle/macos/."
