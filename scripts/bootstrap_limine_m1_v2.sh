#!/bin/bash
set -euo pipefail

echo "🧹 Cleaning old Limine artifacts..."
rm -rf limine

echo "📦 Checking macOS ARM64 deps (brew)..."
brew install autoconf automake libtool nasm mtools || echo "⚠️  Deps already satisfied or install manually"

echo "📥 Step 1: Cloning Limine v7.x-BINARY (prebuilt blobs + Makefile)..."
git clone https://github.com/limine-bootloader/limine.git --branch=v7.x-binary --depth=1 limine

echo "🔨 Step 2: Rebuilding host tool natively for M1 ARM64..."
cd limine
make -j$(sysctl -n hw.logicalcpu)
cd ..

echo "🚀 Step 3: Verifying absolute paths (v7.x-binary layout)..."
# Binaries live directly in root (limine-bios.sys + bios-cd.bin + uefi-cd.bin)
REQUIRED=(
    "limine/limine-bios.sys"
    "limine/limine-bios-cd.bin"
    "limine/limine-uefi-cd.bin"
    "limine/limine"
)

for file in "${REQUIRED[@]}"; do
    if [ ! -f "$file" ]; then
        echo "❌ ERROR: Still missing $file"
        ls -la limine/ | grep -E '\.(sys|bin|exe)$'
        exit 1
    fi
done

echo "✅ LIMINE M1-NATIVE SUBSTRATE READY (v7.x-binary + native limine tool)."
