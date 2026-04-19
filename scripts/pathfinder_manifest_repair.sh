#!/bin/bash
# SexOS SASOS - Phase 18.13: Pathfinder & Manifest Repair
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Repairing Manifest Corruption..."

# Revert the broken dependency added in Phase 18.12
if [ -f "crates/egui-hello/Cargo.toml" ]; then
    echo " -> Purging the bogus sex_rt reference..."
    sed -i.bak '/sex_rt = { path = "..\/..\/crates\/sex_rt" }/d' crates/egui-hello/Cargo.toml
fi

echo "2. Locating the True Sovereign Runtime..."

# Search for the crate that defines itself as the runtime (likely sex-orbclient or sex-rt)
REAL_RT_PATH=$(find . -name "Cargo.toml" -exec grep -l 'name = "sex[-_]rt"' {} \+ | head -n 1 | xargs dirname | sed 's|^\./||')

if [ -z "$REAL_RT_PATH" ]; then
    echo " -> Runtime name 'sex-rt' not found. Searching for 'sex-orbclient'..."
    REAL_RT_PATH=$(find . -name "Cargo.toml" -exec grep -l 'name = "sex-orbclient"' {} \+ | head -n 1 | xargs dirname | sed 's|^\./||')
fi

if [ -n "$REAL_RT_PATH" ]; then
    RT_NAME=$(grep 'name =' "$REAL_RT_PATH/Cargo.toml" | sed 's/name = "//;s/"//')
    echo " -> Found Sovereign Runtime: $RT_NAME at $REAL_RT_PATH"
    
    # Calculate the relative path from egui-hello to the runtime
    # egui-hello is in crates/egui-hello. If RT is in crates/sex-orbclient, relative is ../sex-orbclient
    echo "$RT_NAME = { path = \"../../$REAL_RT_PATH\", default-features = false }" >> crates/egui-hello/Cargo.toml
else
    echo " !! ERROR: No runtime crate (sex-rt or sex-orbclient) found in workspace."
    exit 1
fi

echo "3. Aligning egui-hello Source..."

cat << EOF > crates/egui-hello/src/main.rs
#![no_std]
#![no_main]

extern crate alloc;
use alloc::string::String;
use $RT_NAME; // Linking to the discovered system runtime

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let _greeting = String::from("Hello from the Orbital Userland!");
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
EOF

echo "4. Executing Clean SASOS Synthesis..."

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    --release" > final_pathfinder_build.log 2>&1 || true

echo "--> Build complete. Analyzing logs..."
if grep -q "Finished release" final_pathfinder_build.log; then
    echo "=== PHASE 18.13: MANIFESTS REPAIRED & SYNCED ==="
    echo "Success: Userland is now aligned with the $RT_NAME runtime."
    echo "Next Command: make run-sasos"
else
    echo "BLOCKER REMAINS. Errors in final_pathfinder_build.log:"
    grep "error\[" final_pathfinder_build.log | sort | uniq
    tail -n 15 final_pathfinder_build.log
fi
