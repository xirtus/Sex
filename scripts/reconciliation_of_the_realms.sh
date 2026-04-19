#!/bin/bash
# SexOS SASOS - Phase 18.12: Reconciliation of the Realms
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Redefining egui-hello as a Passive Runtime Consumer..."

# Overwrite egui-hello/src/main.rs with a perfect, conflict-free boilerplate
cat << 'EOF' > crates/egui-hello/src/main.rs
#![no_std]
#![no_main]

extern crate alloc;
use alloc::string::String;
use sex_rt; // The sovereign of the heap

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let _greeting = String::from("Hello from the Orbital Userland!");
    // The heap is managed by sex_rt, so we just use it.
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
EOF

echo "2. Stripping conflicting dependencies from egui-hello/Cargo.toml..."

# Ensure egui-hello doesn't try to pull in its own allocator crate
if [ -f "crates/egui-hello/Cargo.toml" ]; then
    sed -i.bak '/linked-list-allocator/d' crates/egui-hello/Cargo.toml
    sed -i.bak '/linked_list_allocator/d' crates/egui-hello/Cargo.toml
fi

echo "3. Firing GLOBAL CLEAN SYNTHESIS..."

# Purge target to ensure no 'Tranny' or 'Sextranny' artifacts remain in the cache
rm -rf target/

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    rustup component add rust-src && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    --release" > reconciliation_build.log 2>&1 || true

echo "--> Synthesis complete. Analyzing the log..."

if grep -q "Finished release" reconciliation_build.log; then
    echo "=== PHASE 18.12: RECONCILIATION SUCCESSFUL ==="
    echo "The conflict is resolved. Userland is now aligned with sex_rt."
    echo "Next Command: make run-sasos"
else
    echo "BLOCKER REMAINS. Culprits:"
    grep "error\[" reconciliation_build.log | sort | uniq
    tail -n 20 reconciliation_build.log
fi
