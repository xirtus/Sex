#!/bin/bash
set -e

PROJECT_DIR="/Users/xirtus/sites/microkernel"
FILE="$PROJECT_DIR/sex-src/lib/libsys/src/pdx.rs"

# Change this if your function signature uses a different argument name (e.g., 'svc' or 'id')
CORRECT_VAR="name"

echo "============================================================"
echo "[*] Phase 1: Patching pdx.rs Scope Error"
echo "============================================================"

if [ ! -f "$FILE" ]; then
    echo "[-] Error: Could not find $FILE"
    exit 1
fi

echo "[*] Target acquired: $FILE"
echo "[*] Replacing undefined 'service_name' with '$CORRECT_VAR'..."

# Use BSD sed (macOS native) to safely swap the variable in the inline assembly
sed -i '' "s/service_name\.as_ptr()/$CORRECT_VAR.as_ptr()/g" "$FILE"
sed -i '' "s/service_name\.len()/$CORRECT_VAR.len()/g" "$FILE"

echo "[+] Source code patched successfully!"

echo "============================================================"
echo "[*] Phase 2: Re-triggering Native Bare-Metal Build"
echo "============================================================"

cd "$PROJECT_DIR"

# Docker already cached the standard library compilation, so this will be fast
./scripts/clean_build.sh

echo "============================================================"
echo "[+] Pipeline complete. Check output for sexos-v28.0.0.iso"
echo "============================================================"
