#!/bin/bash
# SexOS SASOS - Global Userland Alloc Injector
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Scanning userland for missing alloc preludes..."

# Find all Rust files in servers and sex-packages (if it exists)
find servers -name "*.rs" -type f | while read -r file; do
    # If the file has a #![no_std] directive but NO alloc crate import...
    if grep -q "#!\[no_std\]" "$file" && ! grep -q "extern crate alloc;" "$file"; then
        echo " -> Injecting global alloc prelude into: $file"
        
        # Create a temp file
        echo "#![feature(alloc_error_handler)]" > temp.rs
        echo "extern crate alloc;" >> temp.rs
        echo "use alloc::string::{String, ToString};" >> temp.rs
        echo "use alloc::vec::Vec;" >> temp.rs
        echo "use alloc::boxed::Box;" >> temp.rs
        
        # Append the original file contents
        cat "$file" >> temp.rs
        
        # Overwrite the original file
        mv temp.rs "$file"
    fi
done

# Specifically target the sexbuild main file that panicked
if [ -f "sex-packages/sexbuild/src/main.rs" ]; then
    if ! grep -q "use alloc::string::ToString;" "sex-packages/sexbuild/src/main.rs"; then
        echo " -> Forcing ToString trait into: sex-packages/sexbuild/src/main.rs"
        sed -i.bak '1i\
extern crate alloc;\
use alloc::string::{String, ToString};\
use alloc::vec::Vec;\
use alloc::boxed::Box;\
' "sex-packages/sexbuild/src/main.rs"
    fi
fi

echo "2. Firing global userland build via Docker..."
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "rustup default nightly && cargo build --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release" > global_build.log 2>&1 || true

echo "--> Build complete. Checking status..."
tail -n 20 global_build.log
