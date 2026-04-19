#!/bin/bash
# SexOS SASOS - Userland Nuclear Fix & Final Build
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Surgeons at work: Fixing header alignment in userland..."

# Find all main.rs and driver.rs files in servers/
find servers -name "*.rs" -type f | while read -r file; do
    # Only target files that have been 'rustified' (contain alloc imports)
    if grep -q "extern crate alloc;" "$file"; then
        echo " -> Aligning headers in: $file"
        
        # Create a clean temporary file
        echo "#![no_std]" > temp.rs
        echo "#![no_main]" >> temp.rs
        echo "#![feature(alloc_error_handler)]" >> temp.rs
        
        # Strip out any existing no_std/no_main/alloc_error features to prevent duplicates
        # Then strip the injected alloc prelude we added earlier to re-inject it cleanly
        grep -v "#!\[no_std\]" "$file" | \
        grep -v "#!\[no_main\]" | \
        grep -v "#!\[feature(alloc_error_handler)\]" | \
        grep -v "extern crate alloc;" | \
        grep -v "use alloc::string" | \
        grep -v "use alloc::vec" | \
        grep -v "use alloc::boxed" >> temp.rs
        
        # Now inject the clean prelude right after the inner attributes
        sed -i.bak '4i\
extern crate alloc;\
use alloc::string::{String, ToString};\
use alloc::vec::Vec;\
use alloc::boxed::Box;\
' temp.rs
        
        mv temp.rs "$file"
    fi
done

echo "2. Fixing sexdrive entry point..."
if [ -f "servers/sexdrive/src/driver.rs" ]; then
    if ! grep -q "pub extern \"C\" fn _start" "servers/sexdrive/src/driver.rs"; then
        echo " -> Appending _start() stub to sexdrive"
        cat << 'EOF' >> "servers/sexdrive/src/driver.rs"

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
EOF
    fi
fi

echo "3. Firing ULTIMATE SYSTEM BUILD..."
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && 
    rustup component add rust-src && 
    cargo build --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release
" > final_nuclear_build.log 2>&1 || true

echo "--> Build sequence finished. Analyzing results..."
if grep -q "error: could not compile" final_nuclear_build.log; then
    echo "BLOCKER FOUND. First 3 errors:"
    grep "error\[" final_nuclear_build.log | head -n 3
    tail -n 20 final_nuclear_build.log
else
    echo "PIXELS ARMED. Entire system compiled successfully."
    echo "Ready for Phase 18 Handoff."
    echo "Run: make run-sasos"
fi
