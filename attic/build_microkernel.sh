#!/bin/bash
set -e

PROJECT_DIR="/Users/xirtus/sites/microkernel"
IMAGE_NAME="sex-sasos-builder"

echo "============================================================"
echo "[*] Initializing native ARM64 Rust builder for Intel target"
echo "============================================================"

if [ ! -d "$PROJECT_DIR" ]; then
    echo "[-] Error: Directory $PROJECT_DIR does not exist."
    exit 1
fi

echo "[*] Generating Dockerfile..."
cat << 'DOCKERFILE_EOF' > "$PROJECT_DIR/Dockerfile"
# Run natively on Apple Silicon (ARM64) for maximum speed
FROM rust:slim

ENV DEBIAN_FRONTEND=noninteractive

# Install Make and the utilities required for Limine ISO generation
RUN apt-get update && apt-get install -y \
    make \
    xorriso \
    mtools \
    && rm -rf /var/lib/apt/lists/*

# Add the bare-metal Intel target for Rust cross-compilation
RUN rustup target add x86_64-unknown-none

WORKDIR /src
DOCKERFILE_EOF

echo "[*] Building the native Rust Docker image ($IMAGE_NAME)..."
docker build -t "$IMAGE_NAME" "$PROJECT_DIR"

echo "[*] Mounting source and compiling Sex SASOS for Intel..."
docker run --rm \
    -v "$PROJECT_DIR:/src" \
    "$IMAGE_NAME" \
    bash -c "make clean && make"

echo "============================================================"
echo "[+] Cross-compilation finished flawlessly!"
echo "[+] Ready for deployment to the x17r1 i7 real metal."
echo "============================================================"
