#!/usr/bin/env bash
set -e

echo "→ Building Lopala OS UI Payload..."
cd ui && bun run build && cd ..

echo "→ Building Target: x86_64 (AMD/Intel)"
RUSTFLAGS="-C debuginfo=0" cargo build --release --target x86_64-unknown-linux-musl 2>/dev/null

echo "→ Building Target: aarch64 (ARM64 linux-musl)"
RUSTFLAGS="-C debuginfo=0" cargo build --release --target aarch64-unknown-linux-musl 2>/dev/null

# echo "→ Building Target: x86_64 (Windows .exe)"
# RUSTFLAGS="-C debuginfo=0" cargo build --release --target x86_64-pc-windows-gnu 2>/dev/null

echo "✅ Multi-arch builds completed in ./target/"
