.PHONY: all build run clean help

all: build

build:
	cargo build --package sasos-kernel

run:
	# Note: -cpu max is required for PKU support in QEMU
	cargo run --package sasos-kernel -- -cpu max

clean:
	cargo clean

help:
	@echo "SASOS Build System"
	@echo "  make build - Build the kernel"
	@echo "  make run   - Run the kernel in QEMU (requires cargo-bootimage)"
	@echo "  make clean - Clean build artifacts"
