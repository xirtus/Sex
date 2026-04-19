#!/bin/bash
# SexOS SASOS v1.0.0 - Phase 18.8: Surgical Integrity & Hardened Synthesis
set -euo pipefail

echo "--> 1. Purging 'Sed Residue' and stray handlers from libraries..."
# Use Perl to surgically remove handlers (attribute + body) regardless of line breaks
for lib_file in crates/sex-orbclient/src/lib.rs servers/tuxedo/src/lib.rs; do
    if [ -f "$lib_file" ]; then
        echo "  -> Scrubbing $lib_file"
        # Remove #[panic_handler] functions
        perl -0777 -i -pe 's/\#\[panic_handler\].*?fn.*?\{.*?\}//gs' "$lib_file"
        # Remove #[alloc_error_handler] functions
        perl -0777 -i -pe 's/\#\[alloc_error_handler\].*?fn.*?\{.*?\}//gs' "$lib_file"
        # Remove the specific stray brace at line 24 reported in sex-orbclient
        if [[ "$lib_file" == *"sex-orbclient"* ]]; then
            sed -i '' '24{ /^}$/d; }' "$lib_file"
        fi
    fi
done

echo "--> 2. Executing Hardened Synthesis with all Architectural Flags..."
# We combine -Zjson-target-spec with compiler-builtins-mem for a complete SEAL
docker run --rm -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
    export RUSTFLAGS=\"-A warnings\"
    rustup default nightly
    cargo build --target x86_64-sex.json \
        -Z build-std=core,alloc,compiler_builtins \
        -Z build-std-features=compiler-builtins-mem \
        -Z json-target-spec \
        --release
"

echo "--> 3. Verification check for System Artifacts..."
ls -lh target/x86_64-sex/release/sex-kernel
ls -lh target/x86_64-sex/release/ion-sexshell
ls -lh target/x86_64-sex/release/sexdisplay

echo "=== PHASE 18.8: ARCHITECTURAL SYNTHESIS SUCCESSFUL ==="
