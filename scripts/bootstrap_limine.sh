#!/bin/bash
set -euo pipefail

echo "📥 Step 1: Cloning Limine (v7.x branch)..."
if [ ! -d "limine" ]; then
    git clone https://github.com/limine-bootloader/limine.git --branch=v7.x-binary --depth=1
    echo "✅ Limine binaries cloned."
else
    echo "ℹ️ Limine directory already exists."
fi

echo "🔨 Step 2: Building the Limine deployment tool..."
cd limine
# On macOS, we build the host tool only
make limine
cd ..

echo "🚀 Step 3: Verifying required assets..."
REQUIRED=(
    "limine/limine-bios.sys"
    "limine/limine-cd.bin"
    "limine/limine-eltorito-efi.bin"
    "limine/limine"
)

for file in "${REQUIRED[@]}"; do
    if [ ! -f "$file" ]; then
        echo "❌ ERROR: Missing $file"
        exit 1
    fi
done

echo "✅ LIMINE SUBSTRATE READY."
