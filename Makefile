# SexOS Final Release Build System (v1.0.0)
.PHONY: all build-kernel build-servers initrd limine iso release run-sasos clean

# Configuration
KERNEL_ELF = target/x86_64-unknown-none/debug/sex-kernel
ISO_IMAGE = sexos-v1.0.0.iso
ISO_ROOT = iso_root
LIMINE_VERSION = v7.x-binary
SEXPAC = python3 sex-src/bin/sexpac.py

# Docker wrapper for Apple Silicon / Host safety
DOCKER_IMG = sexos-builder:latest
DOCKER_RUN = docker run --rm -v $(PWD):/sexos -w /sexos $(DOCKER_IMG)

ifeq ($(wildcard /.dockerenv),)
	CARGO = $(DOCKER_RUN) cargo
	CMD_RUN = $(DOCKER_RUN)
else
	CARGO = cargo
	CMD_RUN = 
endif

all: release

build-kernel:
	$(CARGO) build --package sex-kernel --target x86_64-unknown-none

build-servers:
	# Build all standalone servers
	$(CARGO) build --manifest-path servers/sexc/Cargo.toml --target x86_64-unknown-none
	$(CARGO) build --manifest-path servers/sexvfs/Cargo.toml --target x86_64-unknown-none
	$(CARGO) build --manifest-path servers/sexdrives/Cargo.toml --target x86_64-unknown-none
	$(CARGO) build --manifest-path servers/sexinput/Cargo.toml --target x86_64-unknown-none
	$(CARGO) build --manifest-path servers/sexnet/Cargo.toml --target x86_64-unknown-none
	$(CARGO) build --manifest-path servers/sexdisplay/Cargo.toml --target x86_64-unknown-none
	$(CARGO) build --manifest-path servers/sexnode/Cargo.toml --target x86_64-unknown-none
	$(CARGO) build --manifest-path servers/sexstore/Cargo.toml --target x86_64-unknown-none
	$(CARGO) build --manifest-path servers/sexgemini/Cargo.toml --target x86_64-unknown-none

initrd: build-servers
	# Package all SAS artifacts into initrd.sex
	$(CMD_RUN) $(SEXPAC) --out initrd.sex \
		target/x86_64-unknown-none/debug/sexc \
		target/x86_64-unknown-none/debug/sexvfs \
		target/x86_64-unknown-none/debug/sexdrives \
		target/x86_64-unknown-none/debug/sexinput \
		target/x86_64-unknown-none/debug/sexnet \
		target/x86_64-unknown-none/debug/sexdisplay \
		target/x86_64-unknown-none/debug/sexnode \
		target/x86_64-unknown-none/debug/sexstore \
		target/x86_64-unknown-none/debug/sexgemini

limine:
	@if [ ! -d "limine" ]; then \
		git clone https://github.com/limine-bootloader/limine.git --branch=$(LIMINE_VERSION) --depth=1; \
		$(CMD_RUN) make -C limine; \
	fi

iso: build-kernel initrd limine
	mkdir -p $(ISO_ROOT)/boot
	cp $(KERNEL_ELF) $(ISO_ROOT)/boot/sexos-kernel
	cp initrd.sex $(ISO_ROOT)/boot/
	cp limine.cfg limine/limine-bios.sys limine/limine-bios-cd.bin \
	   limine/BOOTX64.EFI limine/BOOTIA32.EFI \
	   $(ISO_ROOT)/
	$(CMD_RUN) xorriso -as mkisofs -b limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot BOOTX64.EFI \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		$(ISO_ROOT) -o $(ISO_IMAGE)
	$(CMD_RUN) ./limine/limine bios-install $(ISO_IMAGE)

release: iso
	@echo "--------------------------------------------------"
	@echo "SexOS Production Release v1.0.0: $(ISO_IMAGE) READY."
	@echo "--------------------------------------------------"

run-sasos: release
	qemu-system-x86_64 -machine q35 -cpu max,pku=on -smp 4 -m 2G \
		-serial stdio -display none \
		-cdrom $(ISO_IMAGE)

clean:
	rm -rf target/
	rm -rf $(ISO_ROOT) $(ISO_IMAGE) initrd.sex limine
