# SexOS Final Release Build System
.PHONY: all build-kernel build-servers initrd iso release run-sasos clean

# Configuration
KERNEL_ELF = target/x86_64-sexos/debug/sex-kernel
ISO_IMAGE = sexos-release.iso
ISO_ROOT = iso_root
LIMINE_VERSION = v7.x-binary
SEXPAC = python3 sex-src/bin/sexpac.py

all: release

build-kernel:
	cargo build --package sex-kernel --target x86_64-unknown-none

build-servers:
	# Build all standalone servers
	cd servers/sexc && cargo build --target x86_64-unknown-none
	cd servers/sexvfs && cargo build --target x86_64-unknown-none
	cd servers/sexdrives && cargo build --target x86_64-unknown-none
	cd servers/sexinput && cargo build --target x86_64-unknown-none
	cd servers/sexnet && cargo build --target x86_64-unknown-none
	cd servers/sexdisplay && cargo build --target x86_64-unknown-none
	cd servers/sexnode && cargo build --target x86_64-unknown-none
	cd servers/sexstore && cargo build --target x86_64-unknown-none
	cd servers/sexgemini && cargo build --target x86_64-unknown-none

initrd: build-servers
	# Package all SAS artifacts into initrd.sex
	$(SEXPAC) --out initrd.sex \
		target/x86_64-unknown-none/debug/sexc \
		target/x86_64-unknown-none/debug/sexvfs \
		target/x86_64-unknown-none/debug/sexdrives \
		target/x86_64-unknown-none/debug/sexinput \
		target/x86_64-unknown-none/debug/sexnet \
		target/x86_64-unknown-none/debug/sexdisplay \
		target/x86_64-unknown-none/debug/sexnode \
		target/x86_64-unknown-none/debug/sexstore \
		target/x86_64-unknown-none/debug/sexgemini \
		sex-src/bin/ash

limine:
	@if [ ! -d "limine" ]; then \
		git clone https://github.com/limine-bootloader/limine.git --branch=$(LIMINE_VERSION) --depth=1; \
		make -C lim; \
	fi

iso: build-kernel initrd limine
	mkdir -p $(ISO_ROOT)/boot
	cp $(KERNEL_ELF) $(ISO_ROOT)/boot/sexos-kernel
	cp initrd.sex $(ISO_ROOT)/boot/
	cp limine.cfg limine/limine-bios.sys limine/limine-bios-cd.bin \
	   limine/limine-uefi-binary/BOOTX64.EFI limine/limine-uefi-binary/BOOTIA32.EFI \
	   $(ISO_ROOT)/
	xorriso -as mkisofs -b limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot BOOTX64.EFI \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		$(ISO_ROOT) -o $(ISO_IMAGE)
	./limine/limine bios-install $(ISO_IMAGE)

release: iso
	@echo "--------------------------------------------------"
	@echo "SexOS Production Release: $(ISO_IMAGE) READY."
	@echo "--------------------------------------------------"

run-sasos: release
	qemu-system-x86_64 -machine q35 -cpu max,pku=on -smp 4 -m 2G \
		-serial stdio -display none \
		-cdrom $(ISO_IMAGE)

clean:
	cargo clean
	rm -rf $(ISO_ROOT) $(ISO_IMAGE) initrd.sex limine
