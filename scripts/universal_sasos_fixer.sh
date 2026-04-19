#!/bin/bash
# SexOS SASOS v1.0.0 - Universal Server Alignment
set -euo pipefail

echo "--> 1. Generating SASOS Perfect Header with Global Allocator..."
cat << 'EOF' > perfect_header.rs
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // In Phase 19, this will be replaced by a PDX call to the kernel 
    // to map the shared heap region.
    loop {}
}
EOF

echo "--> 2. Applying SASOS Standard to ALL servers..."
for server_dir in servers/*; do
    if [ -d "$server_dir/src" ]; then
        MAIN_FILE="$server_dir/src/main.rs"
        LIB_FILE="$server_dir/src/lib.rs"
        
        # Target main.rs for binaries
        if [ -f "$MAIN_FILE" ]; then
            echo "  -> Aligning Binary: $server_dir"
            # Strip existing inner attributes to prevent duplicates
            grep -v "#!\[" "$MAIN_FILE" > body.rs || true
            cat perfect_header.rs body.rs > "$MAIN_FILE"
        fi

        # Target lib.rs for server libraries (like sexfiles)
        if [ -f "$LIB_FILE" ]; then
            echo "  -> Aligning Library: $server_dir"
            if ! grep -q "#!\[no_std\]" "$LIB_FILE"; then
                sed -i '' '1i\
#![no_std]
' "$LIB_FILE"
            fi
        fi
    fi
done

rm perfect_header.rs body.rs

echo "--> 3. Triggering Optimized SASOS Build..."
docker run --rm -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly &&
    cargo build --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release
"

echo "=== PHASE 18.5: ARCHITECTURAL ALIGNMENT COMPLETE ==="
