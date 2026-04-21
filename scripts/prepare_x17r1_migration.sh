#!/bin/bash
set -euo pipefail

echo "📤 Packaging full SexOS repo + limine substrate for x17r1 i7 transfer..."
rm -rf sexos-x17r1-bundle.tar.gz 2>/dev/null || true

# Bundle everything (including the working limine folder we built)
tar -czf sexos-x17r1-bundle.tar.gz \
    --exclude='.git' \
    --exclude='target' \
    --exclude='iso_root' \
    --exclude='sexos-sasos.iso' \
    .

echo "✅ BUNDLE READY: sexos-x17r1-bundle.tar.gz"
echo ""
echo "🚚 Transfer instructions (run on your M1 right now):"
echo "   scp sexos-x17r1-bundle.tar.gz user@your-x17r1-ip:~/"
echo "   (or USB stick / Dropbox / iCloud)"
echo ""
echo "🔧 On your x17r1 i7 (once transferred):"
echo "   1. tar -xzf sexos-x17r1-bundle.tar.gz"
echo "   2. cd sex"
echo "   3. ./scripts/bootstrap_limine_m1_v2.sh     # (works on Intel too)"
echo "   4. ./scripts/fix_iso_synthesis_v7.sh"
echo "   5. qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku -display gtk"
echo ""
echo "💡 Native x86_64 build = no more macOS xorriso quirks. Config WILL be found."
echo "   (PKEY lockdown will show instantly)"
