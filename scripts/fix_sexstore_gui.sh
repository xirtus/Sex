#!/bin/bash
# SexOS SASOS v1.0.0 - sexstore-gui Final Alignment
set -euo pipefail

TARGET_FILE="servers/sexstore-gui/src/main.rs"

echo "--> 1. Surgical Header Alignment for sexstore-gui..."
# Ensure the file exists before patching
if [ -f "$TARGET_FILE" ]; then
    # Strip existing no_std/no_main to prevent duplicates and re-inject at the top
    sed -i '' '/#!\[no_std\]/d' "$TARGET_FILE"
    sed -i '' '/#!\[no_main\]/d' "$TARGET_FILE"
    sed -i '' '/#!\[feature(alloc_error_handler)\]/d' "$TARGET_FILE"
    sed -i '' '/extern crate alloc;/d' "$TARGET_FILE"

    # Create temporary file with correct bare-metal headers
    echo "#![no_std]" > temp_gui.rs
    echo "#![no_main]" >> temp_gui.rs
    echo "#![feature(alloc_error_handler)]" >> temp_gui.rs
    echo "extern crate alloc;" >> temp_gui.rs
    echo "use alloc::string::{String, ToString};" >> temp_gui.rs
    echo "use alloc::vec::Vec;" >> temp_gui.rs
    echo "use alloc::boxed::Box;" >> temp_gui.rs
    
    # Append the rest of the original file
    cat "$TARGET_FILE" >> temp_gui.rs
    mv temp_gui.rs "$TARGET_FILE"
    echo " -> Headers Aligned."
else
    echo "ERR: $TARGET_FILE not found."
    exit 1
fi

echo "--> 2. Ensuring Entry Point and Panic Handler..."
if ! grep -q "pub extern \"C\" fn _start" "$TARGET_FILE"; then
    cat << 'EOF' >> "$TARGET_FILE"

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
EOF
    echo " -> _start Injected."
fi

if ! grep -q "#[panic_handler]" "$TARGET_FILE"; then
    cat << 'EOF' >> "$TARGET_FILE"

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
EOF
    echo " -> Panic Handler Injected."
fi

echo "--> 3. Triggering Clean System Build..."
docker run --rm -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly &&
    rustup component add rust-src &&
    cargo build --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release
"

echo "=== PHASE 18.5: ALL USERLAND SERVERS COMPILED ==="
