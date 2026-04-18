#!/bin/bash
set -e

PROJECT_DIR="/Users/xirtus/sites/microkernel"
FILE="$PROJECT_DIR/sex-src/lib/libsys/src/pdx.rs"
CORRECT_VAR="__________service_name"

echo "============================================================"
echo "[*] Phase 1: Patching 10-Underscore Scope Error"
echo "============================================================"

if [ ! -f "$FILE" ]; then
    echo "[-] Error: Could not find $FILE"
    exit 1
fi

echo "[*] Target acquired: $FILE"
echo "[*] Replacing 'name' with '$CORRECT_VAR'..."

# Swap my incorrect 'name' guess with your actual 10-underscore variable
sed -i '' "s/name\.as_ptr()/$CORRECT_VAR.as_ptr()/g" "$FILE"
sed -i '' "s/name\.len()/$CORRECT_VAR.len()/g" "$FILE"

echo "[+] Source code patched successfully!"

echo "============================================================"
echo "[*] Phase 2: Final Native Bare-Metal Build"
echo "============================================================"

cd "$PROJECT_DIR"

# Firing the final cached build
./scripts/clean_build.sh

echo "============================================================"
echo "[+] Pipeline complete. Your amd64 ISO is ready."
echo "============================================================"
