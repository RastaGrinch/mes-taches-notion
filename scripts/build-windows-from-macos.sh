#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

llvm_bin="$(brew --prefix llvm)/bin"
export PATH="$llvm_bin:$HOME/.cargo/bin:$PATH"

for command in node npm rustc cargo cargo-xwin makensis llvm-rc; do
  command -v "$command" >/dev/null || {
    echo "Commande manquante : $command" >&2
    echo "Consulte https://v2.tauri.app/distribute/windows-installer/#build-windows-apps-on-linux-and-macos" >&2
    exit 1
  }
done

rustup target list --installed | grep -qx x86_64-pc-windows-msvc || \
  rustup target add x86_64-pc-windows-msvc

npm ci
cargo test --manifest-path src-tauri/Cargo.toml

XWIN_CACHE_DIR="$PWD/.xwin-cache" \
  npm run tauri build -- \
  --runner cargo-xwin \
  --target x86_64-pc-windows-msvc \
  --bundles nsis

echo "Terminé. L'installateur se trouve dans src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/."
