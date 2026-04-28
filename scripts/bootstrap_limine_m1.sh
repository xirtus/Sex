#!/bin/bash
set -euo pipefail

echo "🧹 Cleaning old Limine artifacts..."
rm -rf limine

echo "📥 Step 1: Cloning Limine v7.x (Source)..."
git clone https://github.com/limine-bootloader/limine.git --branch=v7.x --depth=1

echo "🔨 Step 2: Building host tool for macOS ARM64..."
cd limine
# This builds the 'limine' executable for your Mac
make limine
cd ..

echo "🚀 Step 3: Verifying absolute paths for synthesis..."
# In v7.x, the binaries are in the root of the limine folder
REQUIRED=(
    "limine/limine-bios.sys"
    "limine/limine-cd.bin"
    "limine/limine-eltorito-efi.bin"
    "limine/limine"
)

for file in "${REQUIRED[@]}"; do
    if [ ! -f "$file" ]; then
        echo "❌ ERROR: Still missing $file"
        exit 1
    fi
done

echo "✅ LIMINE M1 SUBSTRATE READY."
