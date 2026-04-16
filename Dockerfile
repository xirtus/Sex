# ========================================================
# Sex (Single Environment XIPC) - SASOS Microkernel
# Production Build & Emulation Environment (v1.0.0)
# Single-Address-Space Microkernel with PDX ring buffers,
# message-based signals, trampoline thread in sexc, PKU isolation
# ========================================================

FROM rustlang/rust:nightly-bullseye

# Install system dependencies for kernel build + QEMU emulation
RUN apt-get update && apt-get install -y --no-install-recommends \
    qemu-system-x86 \
    lld \
    clang \
    make \
    curl \
    git \
    python3 \
    python3-pip \
    xorriso \
    binutils \
    && rm -rf /var/lib/apt/lists/*

# CRITICAL: Freestanding target for no_std kernel
# This fixes "can't find crate for `core`", bitflags, scopeguard, bit_field, etc.
RUN rustup target add x86_64-unknown-none && \
    rustup component add rust-src llvm-tools-preview && \
    cargo install bootimage cargo-binutils

# Python deps for build scripts (sexpac.py, etc.)
RUN pip3 install --no-cache-dir argparse

# Working directory (must match docker run -w /sexos)
WORKDIR /sexos

# Copy entire project (kernel, Makefile, Limine config, etc.)
COPY . .

# Default command when the container starts
CMD ["make", "build"]

# Metadata for sex-grok / self-repair engine
LABEL description="Sex SASOS Microkernel Build Environment - Lock-free PDX IPC + Message-Based Signals + Intel PKU"
LABEL version="v1.0.0"
LABEL maintainer="Andreas Xirtus"