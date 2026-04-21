#!/bin/bash
set -euo pipefail

BUNDLE="sexos-x17r1-bundle.tar.gz"

echo "🧹 Cleaning old bundle..."
rm -f "$BUNDLE"

echo "📤 Packaging full SexOS repo + limine substrate for x17r1 i7 transfer..."
tar -czf "$BUNDLE" \
    --exclude='.git' \
    --exclude='target' \
    --exclude='iso_root' \
    --exclude='sexos-sasos.iso' \
    --exclude="$BUNDLE" \
    --exclude='*.tar.gz' \
    .

echo "✅ BUNDLE READY: $BUNDLE ($(du -h "$BUNDLE" | cut -f1))"
echo ""
echo "🚚 Transfer instructions (run on your M1 right now):"
echo "   scp $BUNDLE user@your-x17r1-ip:~/"
echo "   (or USB / Dropbox / iCloud)"
echo ""
echo "🔧 On your x17r1 i7 (once transferred):"
echo "   1. tar -xzf $BUNDLE"
echo "   2. cd sex"
echo "   3. ./scripts/bootstrap_limine_m1_v2.sh"
echo "   4. ./scripts/fix_iso_synthesis_v7.sh"
echo "   5. qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku -display gtk"
echo ""
echo "💡 Native Intel build = xorriso/Limine config issues disappear instantly."
echo "   PKEY lockdown will finally show."
