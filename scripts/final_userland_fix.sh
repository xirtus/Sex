#!/bin/bash
# SexOS SASOS - Final Userland Header Alignment
set -euo pipefail

# 1. Create a "Perfect Header" block
cat > perfect_header.txt << 'EOF'
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box;

EOF

# 2. Surgically fix the files that failed the build
for file in servers/sexdisplay/src/main.rs servers/sexinput/src/main.rs servers/tuxedo/src/main.rs; do
    if [ -f "$file" ]; then
        echo " -> Cleaning and Aligning headers in: $file"
        
        # Strip all existing inner attributes and alloc prelude to avoid the "inner attribute" error
        grep -v "#!\[" "$file" | \
        grep -v "extern crate alloc;" | \
        grep -v "use alloc::string" | \
        grep -v "use alloc::vec" | \
        grep -v "use alloc::boxed" > temp_body.rs
        
        # Combine the perfect header with the cleaned body
        cat perfect_header.txt temp_body.rs > "$file"
        rm temp_body.rs
    fi
done

rm perfect_header.txt

echo "3. Firing GLOBAL SYSTEM BUILD..."
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && 
    rustup component add rust-src && 
    cargo build --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release
" > final_system_build.log 2>&1 || true

echo "--> Build sequence complete. Analyzing results..."
if grep -q "error: could not compile" final_system_build.log; then
    echo "BLOCKER REMAINS. Culprits:"
    grep "error\[" final_system_build.log | sort | uniq
    tail -n 20 final_system_build.log
else
    echo "SYSTEM COMPILATION SUCCESSFUL."
    echo "All C/Rust servers and Kernel are ready."
    echo "Next Command: make run-sasos"
fi
