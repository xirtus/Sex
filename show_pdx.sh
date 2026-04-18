#!/bin/bash
PROJECT_DIR="/Users/xirtus/sites/microkernel"
FILE="$PROJECT_DIR/sex-src/lib/libsys/src/pdx.rs"

echo "============================================================"
echo "[*] Extracting the function signature around line 88..."
echo "============================================================"

if [ ! -f "$FILE" ]; then
    echo "[-] Error: Could not find $FILE"
    exit 1
fi

# Print lines 75 through 95 to get the full context of the function
sed -n '75,95p' "$FILE"

echo "============================================================"
