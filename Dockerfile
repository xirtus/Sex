# SexOS Production Build & Emulation Environment
FROM rustlang/rust:nightly-bullseye

# Install dependencies for microkernel development and SASOS emulation
RUN apt-get update && apt-get install -y \
    qemu-system-x86 \
    lld \
    clang \
    make \
    curl \
    git \
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Install rust-src component and bootimage tool
RUN rustup component add rust-src && \
    cargo install bootimage

# Setup environment for sexpac.py
RUN pip3 install --no-cache-dir argparse

# Set work directory
WORKDIR /sexos

# Copy the entire microkernel project
COPY . .

# Default command: build the bootable image and provide a ready environment
# To test in Docker: docker build -t sexos . && docker run --privileged sexos make run-sasos
CMD ["make", "build"]
