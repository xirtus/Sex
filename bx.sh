#!/usr/bin/env bash
# bx - Build & X-Run (Patched QEMU)
# The easiest way to rebuild SexOS and test the new USB stack.

set -e

echo "🛠️  Building SexOS Native..."
./scripts/entrypoint_build.sh

echo "✅ Build Success. Launching Patched QEMU..."
./qemuX.sh "$@"
