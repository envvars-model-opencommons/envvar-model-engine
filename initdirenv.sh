#!/usr/bin/env bash

echo "🧹 Cleaning up old build artifacts..."
# Remove all bin and obj folders recursively
find . -type d \( -name "bin" -eq -name "obj" \) -exec rm -rf {} + 2>/dev/null

echo "♻️  Reloading Direnv..."
direnv allow
direnv reload

echo "✨ Project cleaned and environment refreshed."