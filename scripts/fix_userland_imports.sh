#!/bin/bash
# SexOS SASOS - Userland Build Isolation & Cleanup
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Cleaning up kernel feature warnings..."
# macOS sed: remove the unused allocator_api feature
sed -i.bak 's/#\!\[feature(abi_x86_interrupt, allocator_api)\]/#\!\[feature(abi_x86_interrupt)\]/' kernel/src/lib.rs

echo "2. Injecting missing alloc imports into sexbuild..."
# If sexbuild has a main.rs, ensure alloc and ToString are available
if [ -f "servers/sexbuild/src/main.rs" ]; then
    # Create a temporary file with the necessary imports at the top
    echo "#![feature(alloc_error_handler)]" > temp_main.rs
    echo "extern crate alloc;" >> temp_main.rs
    echo "use alloc::string::ToString;" >> temp_main.rs
    
    # Strip existing duplicate headers if they exist to prevent conflicts, then append the rest of the file
    grep -v "extern crate alloc;" servers/sexbuild/src/main.rs | grep -v "use alloc::string::ToString;" >> temp_main.rs
    
    mv temp_main.rs servers/sexbuild/src/main.rs
    echo " -> Injected alloc::string::ToString into sexbuild/src/main.rs"
fi

echo "3. Running isolated build for sexbuild..."
# Note the added -Z json-target-spec
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "rustup default nightly && cargo build --package sexbuild --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release" > userland_build.log 2>&1 || true

echo "--> Check the top of userland_build.log for the root cause:"
head -n 30 userland_build.log
