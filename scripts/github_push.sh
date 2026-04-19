#!/bin/bash
set -euo pipefail

echo "🧹 Final cleanup before push..."
# We keep the scripts but remove the extracted ELF and ISO to keep the repo lean
rm -f ./sex-kernel.elf ./sexos-sasos.iso
rm -rf ./iso_root

echo "🌿 Preparing Git Commit..."
git add .
git commit -m "Phase 18.31-52: Baseline Stability Achieved
- Resolved Hyphen/Underscore crate deadlock.
- Implemented HHDM Shield for Higher-Half Memory Allocation.
- Reconciled Limine v0.x API fractures (MemmapRequest/type_).
- Successfully minted bootable ISO on M1/M2 Mac."

echo "🚀 Pushing to GitHub..."
git push origin main
