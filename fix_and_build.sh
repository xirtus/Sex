#!/bin/bash
set -e

PROJECT_DIR="/Users/xirtus/sites/microkernel"

echo "============================================================"
echo "[*] Phase 1: Hunting and Patching the Dockerfile"
echo "============================================================"

cd "$PROJECT_DIR"

# Locate the active Dockerfile
if [ -f "scripts/Dockerfile" ]; then
    DFILE="scripts/Dockerfile"
elif [ -f "Dockerfile" ]; then
    DFILE="Dockerfile"
else
    echo "[-] Error: Could not find Dockerfile in root or scripts/ directory."
    exit 1
fi

echo "[*] Target acquired: $DFILE"

# Check if we already applied the patch to avoid duplicate lines
if grep -q "rust-src" "$DFILE"; then
    echo "[!] rust-src is already present in $DFILE. Skipping patch."
else
    echo "[*] Injecting 'rust-src' requirement for bare-metal compilation..."
    
    # Safely append the fix. The OR (||) fallbacks ensure it installs successfully 
    # regardless of exactly how your base image originally installed nightly Rust.
    cat << 'PATCH_EOF' >> "$DFILE"

# --- Auto-Injected Fix for custom target x86_64-sex.json ---
# Cargo needs the raw standard library source to compile 'core' for bare-metal
RUN rustup component add rust-src --toolchain nightly-aarch64-unknown-linux-gnu || \
    rustup component add rust-src --toolchain nightly || \
    rustup component add rust-src
PATCH_EOF
    
    echo "[+] Dockerfile patched successfully!"
fi

echo "============================================================"
echo "[*] Phase 2: Triggering Native M1 Docker Build"
echo "============================================================"

# Because we modified the Dockerfile, Docker will automatically invalidate 
# the cache for this step and rebuild the image with the new source code.
./scripts/clean_build.sh

echo "============================================================"
echo "[+] Pipeline complete. Check output for sexos-v28.0.0.iso"
echo "============================================================"
