#!/bin/bash
set -e

echo "[hyprnav] Cleaning previous build..."
cargo clean

echo "[hyprnav] Building optimized release binary..."
cargo build --release

echo "[hyprnav] Stripping binary to reduce size..."
strip target/release/hyprnav || true

mkdir -p dist

echo "[hyprnav] Copying binary to dist/ ..."
cp target/release/hyprnav dist/hyprnav

echo ""
echo "========================================"
echo "  Build complete!"
echo "  Binary available at: dist/hyprnav"
echo "========================================"
