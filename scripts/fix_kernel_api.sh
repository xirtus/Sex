#!/bin/bash
set -e

echo "--- 1. Aligning Kernel API & Balancing Braces ---"

# Fix Limine API points
sed -i '' 's/MemoryMapRequest/MemmapRequest/g' kernel/src/main.rs 2>/dev/null || true
sed -i '' 's/\.get_response()/.response()/g' kernel/src/main.rs 2>/dev/null || true
sed -i '' 's/framebuffers().next()/framebuffers().iter().next()/g' kernel/src/main.rs 2>/dev/null || true

# Fix Framebuffer Hybrid API
sed -i '' 's/fb\.width()/fb.width/g' kernel/src/main.rs 2>/dev/null || true
sed -i '' 's/fb\.height()/fb.height/g' kernel/src/main.rs 2>/dev/null || true
sed -i '' 's/fb\.pitch()/fb.pitch/g' kernel/src/main.rs 2>/dev/null || true
sed -i '' 's/fb\.address\([^(]\)/fb.address()\1/g' kernel/src/main.rs 2>/dev/null || true

# Fix Pointers & Delimiters
sed -i '' 's/address.as_ptr().unwrap()/address/g' kernel/src/main.rs 2>/dev/null || true
sed -i '' 's/syscall_entry as u64/syscall_entry as *const () as u64/g' kernel/src/interrupts.rs 2>/dev/null || true

# REMOVE THE ORPHANED BRACE: 
# The compiler flagged line 94 as an unexpected delimiter. We delete it to balance the file.
sed -i '' '94d' kernel/src/main.rs 2>/dev/null || true

echo "--- 2. Building Kernel with compiler-builtins-mem ---"
# Added CARGO_UNSTABLE_BUILD_STD_FEATURES=compiler-builtins-mem to provide memcpy/memset
docker run --platform linux/amd64 --rm -v $(pwd):/sex -w /sex \
-e CARGO_UNSTABLE_JSON_TARGET_SPEC=true \
-e CARGO_UNSTABLE_BUILD_STD=core,alloc \
-e CARGO_UNSTABLE_BUILD_STD_FEATURES=compiler-builtins-mem \
--entrypoint /bin/bash sexos-builder:v28 -c "
    rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu && \
    cargo +nightly build \
    --package sex-kernel \
    --target x86_64-sex.json \
    --release \
    --config \"target.x86_64-sex.rustflags=['-C', 'linker=rust-lld', '-C', 'target-cpu=skylake', '-C', 'link-arg=--script=kernel/linker.ld', '-C', 'code-model=kernel', '-C', 'relocation-model=static']\"
"

echo "=========================================="
echo " PHASE 18: KERNEL LINKED & SYMBOLS RESOLVED"
echo "=========================================="
