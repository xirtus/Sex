#!/bin/bash
set -e
# gen_initrd.sh: Package all SexOS SPD artifacts into the final initrd.sex

BINPKGS="binpkgs"
OUT="initrd.sex"
SEXPAC="python3 sex-src/bin/sexpac.py"

echo "gen_initrd: Bundling SPD packages from $BINPKGS..."

# Get all .spd files from binpkgs/
SPD_FILES=$(find "$BINPKGS" -name "*.spd")

if [ -z "$SPD_FILES" ]; then
    echo "Error: No SPD packages found in $BINPKGS. Run sexbuild first."
    exit 1
fi

# Run sexpac to generate initrd.sex
$SEXPAC --out "$OUT" $SPD_FILES

echo "gen_initrd: SUCCESS. Created $OUT"
