#!/bin/bash
# SexOS SASOS v1.0.0 - Phase 18.7: Custom Target Specification Fix
set -euo pipefail

echo "--> 1. Executing Synthesis with Full Unstable Flag Stack..."
# We must pass -Zjson-target-spec to allow the use of x86_64-sex.json
docker run --rm -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
    export RUSTFLAGS=\"-A warnings\"
    rustup default nightly
    cargo build --target x86_64-sex.json \
        -Z build-std=core,alloc,compiler_builtins \
        -Z build-std-features=compiler-builtins-mem \
        -Z json-target-spec \
        --release
"

echo "--> 2. Verifying ELF Artifacts in Target Directory..."
ls -lh target/x86_64-sex/release/sex-kernel
ls -lh target/x86_64-sex/release/ion-sexshell
ls -lh target/x86_64-sex/release/egui-hello

echo "=== PHASE 18.7: BUILD PIPELINE FULLY CONVERGED ==="
