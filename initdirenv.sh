#!/usr/bin/env bash
#
# One-shot setup and refresh for the argenv dev environment.
#
#     bash initdirenv.sh
#
# Run it once after cloning, and any time you want a clean rebuild. It is
# idempotent and safe to re-run.
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$root"

nixflags=(--extra-experimental-features "nix-command flakes")

command -v nix >/dev/null 2>&1 || {
    echo "❌ nix not found — install Nix with flakes enabled first." >&2
    exit 1
}
command -v direnv >/dev/null 2>&1 || {
    echo "❌ direnv not found — install it and hook it into your shell." >&2
    exit 1
}

echo "🧹 Cleaning build artifacts..."
if command -v cargo >/dev/null 2>&1; then
    cargo clean 2>/dev/null || true
else
    rm -rf target
fi
find . -maxdepth 2 -type l \( -name 'result' -o -name 'result-*' \) -exec rm -f {} + 2>/dev/null || true

echo "🔒 Locking the dev flake (creates or refreshes dev/flake.lock)..."
nix "${nixflags[@]}" flake lock ./dev

echo "✅ Allowing direnv for this project..."
direnv allow "$root"

echo "🏗️  Building the dev shell (the first run downloads the editor and extensions)..."
direnv exec "$root" true

echo "✨ Ready — open the isolated editor with:  code .   (or: code-dev .)"
echo "   The environment auto-loads at your next prompt here, or run: direnv reload"
