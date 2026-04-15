# SexOS Build System - SASOS Pipeline
.PHONY: all build-kernel build-apps initrd iso run-sasos clean help

# Configuration
KERNEL_ELF = target/x86_64-sexos/debug/sex-kernel
ISO_IMAGE = sexos.iso
ISO_ROOT = iso_root
LIMINE_VERSION = v7.x-binary
SEXPAC = python3 sex-src/bin/sexpac.py

all: iso

build-kernel:
	cargo build --package sex-kernel

build-apps:
	make -C sex-src/bin -f Makefile.apps

initrd: build-apps
	$(SEXPAC) --out initrd.sex \
		sex-src/bin/sexit.sex \
		sex-src/bin/sext.sex \
		sex-src/bin/sexvfs.sex \
		sex-src/bin/sexc.sex

limine:
	git clone https://github.com/limine-bootloader/limine.git --branch=$(LIMINE_VERSION) --depth=1
	make -C limine

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

run-sasos: iso
	# Launch QEMU with PKU support enabled for Single Address Space isolation.
	qemu-system-x86_64 -machine q35 -cpu max,pku=on -smp 4 -m 2G \
		-serial stdio -display none \
		-cdrom $(ISO_IMAGE)

clean:
	cargo clean
	make -C sex-src/bin -f Makefile.apps clean
	rm -rf $(ISO_ROOT) $(ISO_IMAGE) initrd.sex limine

help:
	@echo "SexOS Build System (SASOS)"
	@echo "  make iso       - Generate bootable sexos.iso with Limine and Initrd"
	@echo "  make run-sasos - Build and boot SexOS in QEMU with PKU"
	@echo "  make clean     - Clean all build artifacts"
