#!/bin/bash
# SexOS SASOS - Userland Straggler Cleanup
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Fixing sexgemini missing bare-metal entry point..."
if [ -f "servers/sexgemini/src/main.rs" ]; then
    # If it doesn't have #![no_main], inject it at the very top
    if ! grep -q "#!\[no_main\]" "servers/sexgemini/src/main.rs"; then
        sed -i.bak '1i\
#![no_main]
' "servers/sexgemini/src/main.rs"
        echo " -> Injected #![no_main] into sexgemini"
    fi
    
    # If it doesn't have a _start function, append a dummy infinite loop
    if ! grep -q "pub extern \"C\" fn _start" "servers/sexgemini/src/main.rs"; then
        cat << 'EOF' >> "servers/sexgemini/src/main.rs"

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
EOF
        echo " -> Appended _start() stub to sexgemini"
    fi
fi

echo "2. Isolating 'tuxedo' library errors via Docker..."
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "rustup default nightly && cargo build --package tuxedo --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release" > tuxedo_build.log 2>&1 || true

echo "--> Displaying exact tuxedo errors:"
grep -B 2 -A 10 "error\[" tuxedo_build.log | head -n 40
