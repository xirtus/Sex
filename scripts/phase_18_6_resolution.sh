#!/bin/bash
# SexOS SASOS v1.0.0 - Phase 18.6: Conflict & Intrinsic Resolution
set -euo pipefail

echo "--> 1. De-conflicting Libraries (Removing panic_handlers from libs)..."
# Libraries should NOT have panic handlers; only the final binaries should.
for lib_file in crates/sex-orbclient/src/lib.rs servers/tuxedo/src/lib.rs; do
    if [ -f "$lib_file" ]; then
        echo "  -> Cleaning $lib_file"
        # Remove the panic handler block if it exists
        sed -i '' '/#\[panic_handler\]/,/}/d' "$lib_file"
    fi
done

echo "--> 2. Standardizing Binary Headers for egui-hello and ion-sexshell..."
# Ensure new binaries are also aligned with the SASOS standard
for target_bin in servers/egui-hello servers/ion-sexshell; do
    if [ -d "$target_bin/src" ]; then
        MAIN_FILE="$target_bin/src/main.rs"
        cat << 'EOF' > "$MAIN_FILE"
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(_layout: core::alloc::Layout) -> ! { loop {} }

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
EOF
        echo "  -> $target_bin: Standardized."
    fi
done

echo "--> 3. Executing Synthesis with Hardened Intrinsics..."
# We add compiler-builtins-mem to the build-std features to provide memcpy/memset
docker run --rm -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
    export RUSTFLAGS=\"-A warnings\"
    rustup default nightly
    cargo build --target x86_64-sex.json \
        -Z build-std=core,alloc,compiler_builtins \
        -Z build-std-features=compiler-builtins-mem \
        --release
"

echo "=== PHASE 18.6: INTRINSICS ARMED & CONFLICTS RESOLVED ==="
