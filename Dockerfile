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

# --- Auto-Injected Fix for custom target x86_64-sex.json ---
# Cargo needs the raw standard library source to compile 'core' for bare-metal
RUN rustup component add rust-src --toolchain nightly-aarch64-unknown-linux-gnu || \
    rustup component add rust-src --toolchain nightly || \
    rustup component add rust-src
