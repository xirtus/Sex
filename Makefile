# SexOS Phase 28 Build System (v28.0.0)
.PHONY: all build-kernel build-servers initrd limine iso release run-sasos clean \
	docker-build docker-run docker-full-test docker-full-test-x86 docker-cross-build cross-build

# Configuration
KERNEL_ELF = target/x86_64-sex/release/sex-kernel
ISO_IMAGE = sexos-v28.0.0.iso
ISO_ROOT = iso_root
LIMINE_VERSION = v7.x-binary
SEXPAC = python3 sex-src/bin/sexpac.py

# Phase 28 Strict Cross-Compilation Flags
RUSTFLAGS_KERNEL = -C linker=sex-ld -C target-cpu=skylake -C link-arg=--script=kernel/linker.ld -C code-model=kernel -C relocation-model=static
RUSTFLAGS_SERVER = -C linker=sex-ld
CARGO_FLAGS_KERNEL = --package sex-kernel --target x86_64-sex.json --release
CARGO_FLAGS_SERVER = --target x86_64-sex.json --release

# Docker wrapper
DOCKER_FLAGS = --platform linux/amd64 --rm -v $(PWD):/sex -w /sex --privileged -v /dev/kvm:/dev/kvm -v $(PWD)/sexshop:/sex/shop
DOCKER_IMG = sexos-builder:v28

# Check if inside Docker
ifeq ($(wildcard /.dockerenv),)
	CARGO = docker run $(DOCKER_FLAGS) --entrypoint cargo $(DOCKER_IMG) +nightly -Zjson-target-spec
	CMD_RUN = docker run $(DOCKER_FLAGS) --entrypoint "" $(DOCKER_IMG)
	DOCKER_RUN = docker run $(DOCKER_FLAGS) --entrypoint make $(DOCKER_IMG)
	LIMINE_BIN = ./limine/limine
	PATH_MOD = 
else
	CARGO = cargo +nightly -Zjson-target-spec
	CMD_RUN = 
	DOCKER_RUN = make
	LIMINE_BIN = /opt/limine/limine
	PATH_MOD = export PATH=./scripts:./target/release:$$PATH &&
endif

all: docker-release

docker-release:
	./scripts/clean_build.sh

# Task 2: Cross Tool Integration
cross-build: docker-cross-build

docker-cross-build:
	docker build --platform linux/amd64 -t sex-builder .
	docker run --rm -v $(PWD):/sex --entrypoint bash sex-builder -c \
		"set -e && \
		echo '=== Building SASOS Workspace (x86_64-unknown-none) ===' && \
		RUSTFLAGS='-C target-cpu=skylake -C linker=rust-lld -C link-arg=--script=kernel/linker.ld' \
		cargo build -Z build-std=core,alloc \
			--target x86_64-unknown-none \
			--release \
			--workspace \
			--exclude sexbuild \
			--exclude egui-hello"
			
			
			
			   
docker-full-test-x86: docker-build
	$(DOCKER_RUN) release
	$(DOCKER_RUN) run-sasos

build-linker:
	@chmod +x scripts/sex-ld
	@if [ ! -f "scripts/sex-ld" ]; then echo "ERROR: scripts/sex-ld missing"; exit 1; fi

build-kernel: build-linker
	$(PATH_MOD) RUSTFLAGS="$(RUSTFLAGS_KERNEL)" $(CARGO) build $(CARGO_FLAGS_KERNEL)

build-servers: build-linker
	# Build all standalone servers
	$(PATH_MOD) RUSTFLAGS="$(RUSTFLAGS_SERVER)" $(CARGO) build --manifest-path servers/sexc/Cargo.toml $(CARGO_FLAGS_SERVER)
	$(PATH_MOD) RUSTFLAGS="$(RUSTFLAGS_SERVER)" $(CARGO) build --manifest-path servers/sexfiles/Cargo.toml $(CARGO_FLAGS_SERVER)
	$(PATH_MOD) RUSTFLAGS="$(RUSTFLAGS_SERVER)" $(CARGO) build --manifest-path servers/sexdrive/Cargo.toml $(CARGO_FLAGS_SERVER)
	$(PATH_MOD) RUSTFLAGS="$(RUSTFLAGS_SERVER)" $(CARGO) build --manifest-path servers/tuxedo/Cargo.toml $(CARGO_FLAGS_SERVER)
	$(PATH_MOD) RUSTFLAGS="$(RUSTFLAGS_SERVER)" $(CARGO) build --manifest-path servers/sexinput/Cargo.toml $(CARGO_FLAGS_SERVER)
	$(PATH_MOD) RUSTFLAGS="$(RUSTFLAGS_SERVER)" $(CARGO) build --manifest-path servers/sexnet/Cargo.toml $(CARGO_FLAGS_SERVER)
	$(PATH_MOD) RUSTFLAGS="$(RUSTFLAGS_SERVER)" $(CARGO) build --manifest-path servers/sexdisplay/Cargo.toml $(CARGO_FLAGS_SERVER)
	$(PATH_MOD) RUSTFLAGS="$(RUSTFLAGS_SERVER)" $(CARGO) build --manifest-path servers/sexnode/Cargo.toml $(CARGO_FLAGS_SERVER)
	$(PATH_MOD) RUSTFLAGS="$(RUSTFLAGS_SERVER)" $(CARGO) build --manifest-path servers/sexstore/Cargo.toml $(CARGO_FLAGS_SERVER)
	$(PATH_MOD) RUSTFLAGS="$(RUSTFLAGS_SERVER)" $(CARGO) build --manifest-path servers/sexgemini/Cargo.toml $(CARGO_FLAGS_SERVER)
	$(PATH_MOD) RUSTFLAGS="$(RUSTFLAGS_SERVER)" $(CARGO) build --manifest-path servers/sexshop/Cargo.toml $(CARGO_FLAGS_SERVER)
	$(PATH_MOD) RUSTFLAGS="$(RUSTFLAGS_SERVER)" $(CARGO) build --manifest-path sex-packages/ion-sexshell/Cargo.toml $(CARGO_FLAGS_SERVER)

initrd: build-servers
	# Package core SAS artifacts into initrd.sex
	$(CMD_RUN) $(SEXPAC) --out initrd.sex \
		target/x86_64-sex/release/sexc \
		target/x86_64-sex/release/sexfiles \
		target/x86_64-sex/release/sexdrive \
		target/x86_64-sex/release/tuxedo \
		target/x86_64-sex/release/sexinput \
		target/x86_64-sex/release/sexnet \
		target/x86_64-sex/release/sexdisplay \
		target/x86_64-sex/release/sexnode \
		target/x86_64-sex/release/sexstore \
		target/x86_64-sex/release/sexgemini \
		target/x86_64-sex/release/sexshop \
		target/x86_64-sex/release/ion-sexshell

limine:
	@if [ ! -d "limine" ] && [ ! -f "/opt/limine/limine" ]; then \
		git clone https://github.com/limine-bootloader/limine.git --branch=$(LIMINE_VERSION) --depth=1; \
		$(CMD_RUN) make -C limine; \
	fi

iso: build-kernel initrd limine
	mkdir -p $(ISO_ROOT)/boot
	cp $(KERNEL_ELF) $(ISO_ROOT)/boot/sexos-kernel
	cp initrd.sex $(ISO_ROOT)/boot/
	@if [ -d "/opt/limine" ]; then \
		cp /opt/limine/limine-bios.sys /opt/limine/limine-bios-cd.bin \
		   /opt/limine/BOOTX64.EFI /opt/limine/BOOTIA32.EFI \
		   $(ISO_ROOT)/; \
	else \
		cp limine/limine-bios.sys limine/limine-bios-cd.bin \
		   limine/BOOTX64.EFI limine/BOOTIA32.EFI \
		   $(ISO_ROOT)/; \
	fi
	cp limine.cfg $(ISO_ROOT)/
	$(CMD_RUN) xorriso -as mkisofs -b limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot BOOTX64.EFI \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		$(ISO_ROOT) -o $(ISO_IMAGE)
	$(CMD_RUN) $(LIMINE_BIN) bios-install $(ISO_IMAGE)

release: iso
	@echo "--------------------------------------------------"
	@echo "SexOS Production Release v28.0.0: $(ISO_IMAGE) READY."
	@echo "--------------------------------------------------"

run-sasos: release
	./run_sasos.sh

docker-build:
	docker build --platform linux/amd64 -t $(DOCKER_IMG) .

docker-run:
	$(DOCKER_RUN) run-sasos

docker-full-test: docker-build
	$(DOCKER_RUN) release
	$(DOCKER_RUN) run-sasos

clean:
	rm -rf target/
	rm -rf $(ISO_ROOT) $(ISO_IMAGE) initrd.sex limine

verify-abi:
	@! readelf -A target/x86_64-unknown-none/release/kernel | grep -i "soft-float" || (echo "ABI Mismatch Detected!" && exit 1)
