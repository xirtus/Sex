# ========================================================
# Sex (Single Environment XIPC) - SASOS Microkernel
# Phase 28: Strict x86_64 Cross-Compilation Pipeline
# ========================================================

FROM --platform=linux/amd64 rustlang/rust:nightly-bullseye

# Install system dependencies for kernel build + QEMU emulation
# Includes limine, make, build-essential, qemu-system-x86_64, xorriso
RUN apt-get update && apt-get install -y --no-install-recommends \
    qemu-system-x86 \
    lld \
    clang \
    make \
    build-essential \
    curl \
    git \
    python3 \
    python3-pip \
    xorriso \
    binutils \
    && rm -rf /var/lib/apt/lists/*

# Install limine binaries (v7.x)
RUN git clone https://github.com/limine-bootloader/limine.git --branch=v7.x-binary --depth=1 /opt/limine && \
    make -C /opt/limine

# Rust setup (2024 Edition)
RUN rustup component add rust-src llvm-tools-preview && \
    rustup target add x86_64-unknown-none && \
    cargo install cargo-binutils bootimage

COPY x86_64-sex.json /usr/local/rustup/targets/x86_64-unknown-none.json
ENV RUSTFLAGS="-C target-cpu=skylake -C linker=sex-ld -C link-arg=--script=kernel/linker.ld -C code-model=kernel -C relocation-model=static"
ENV CARGO_BUILD_TARGET="x86_64-unknown-none"

# Working directory
WORKDIR /sex

# Copy entire project
COPY . .

# sexshop persistent volume mount point
RUN mkdir -p /sex/shop

# Entrypoints for build and run
ENTRYPOINT ["/usr/bin/make"]
CMD ["build"]

# Metadata
LABEL description="Sex SASOS Microkernel Phase 28 - Strict x86_64 Cross-Build Pipeline"
LABEL version="v28.0.0"
LABEL maintainer="Andreas Xirtus"