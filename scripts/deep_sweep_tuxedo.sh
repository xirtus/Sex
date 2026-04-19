#!/bin/bash
# SexOS SASOS - Deep Sweep Extinction Protocol
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Scanning entire repository for 'tuxedo' or 'sextuxedo'..."

# Find all files containing 'tuxedo' (case-insensitive), ignoring standard build/git dirs
# We also ignore .bak files so we don't accidentally modify previous backups
FIND_RESULTS=$(grep -rilE 'tuxedo' . \
    --exclude-dir={.git,target,build_backups,build_error_logs} \
    --exclude="*.bak" || true)

if [ -z "$FIND_RESULTS" ]; then
    echo " -> CLEAN! No lingering references to 'sextuxedo' or 'tuxedo' were found."
else
    echo " -> Found lingering references in:"
    echo "$FIND_RESULTS" | sed 's/^/    - /'
    echo ""
    echo "2. Eradicating extinct references..."
    
    for file in $FIND_RESULTS; do
        # Replace 'sextuxedo' with 'tuxedo'
        sed -i.bak 's/sextuxedo/tuxedo/g' "$file"
        sed -i.bak 's/Sextuxedo/Tuxedo/g' "$file"
        
        # Replace remaining 'tuxedo' with 'tuxedo'
        sed -i.bak 's/tuxedo/tuxedo/g' "$file"
        sed -i.bak 's/Tuxedo/Tuxedo/g' "$file"
        
        echo "   [PATCHED] $file"
    done
    echo " -> Deep sweep complete. All references modernized to 'tuxedo'."
fi

echo "3. Re-verifying Tuxedo build..."
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "rustup default nightly && cargo build --package tuxedo --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release" > tuxedo_final_build.log 2>&1 || true

echo "--> Build output for tuxedo:"
tail -n 15 tuxedo_final_build.log
