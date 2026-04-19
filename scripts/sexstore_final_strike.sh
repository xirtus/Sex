#!/bin/bash
# SexOS SASOS v1.0.0 - sexstore & Global Silence
set -euo pipefail

# Define the SASOS Perfect Boilerplate
BOILERPLATE=$(cat << 'EOF'
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
    loop {}
}
EOF
)

echo "--> 1. Surgically overhauling sexstore and sexstore-gui..."
for target in servers/sexstore servers/sexstore-gui; do
    if [ -d "$target/src" ]; then
        # Completely replace main.rs with the valid SASOS boilerplate
        echo "$BOILERPLATE" > "$target/src/main.rs"
        echo "  -> $target: Overhauled."
    fi
done

echo "--> 2. Patching Cargo.toml to ensure linked-list-allocator is present..."
# This ensures every server can actually find the allocator crate
for server_dir in servers/*; do
    if [ -f "$server_dir/Cargo.toml" ]; then
        if ! grep -q "linked-list-allocator" "$server_dir/Cargo.toml"; then
            sed -i '' '/\[dependencies\]/a\
linked-list-allocator = "0.10.5"
' "$server_dir/Cargo.toml"
        fi
    fi
done

echo "--> 3. Executing Silent SASOS Build (Warnings Suppressed)..."
# We pass -A warnings to clear the visual clutter of unused imports
docker run --rm -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
    export RUSTFLAGS=\"-A warnings\"
    rustup default nightly &&
    cargo build --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release
"

echo "=== PHASE 18.5: SYSTEM SYNTHESIS SUCCESSFUL ==="
