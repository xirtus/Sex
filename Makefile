CC ?= cc
STRIP ?= strip
INSTALL ?= ./install-sh

PREFIX ?= /usr/local

CFLAGS ?= -g -O2 -pipe

.PHONY: all
all: limine

.PHONY: install
install: all
	$(INSTALL) -d '$(DESTDIR)$(PREFIX)/share'
	$(INSTALL) -d '$(DESTDIR)$(PREFIX)/share/limine'
	$(INSTALL) -m 644 limine-bios.sys '$(DESTDIR)$(PREFIX)/share/limine/'
	$(INSTALL) -m 644 limine-bios-cd.bin '$(DESTDIR)$(PREFIX)/share/limine/'
	$(INSTALL) -m 644 limine-uefi-cd.bin '$(DESTDIR)$(PREFIX)/share/limine/'
	$(INSTALL) -m 644 limine-bios-pxe.bin '$(DESTDIR)$(PREFIX)/share/limine/'
	$(INSTALL) -m 644 BOOTX64.EFI '$(DESTDIR)$(PREFIX)/share/limine/'
	$(INSTALL) -m 644 BOOTIA32.EFI '$(DESTDIR)$(PREFIX)/share/limine/'
	$(INSTALL) -m 644 BOOTAA64.EFI '$(DESTDIR)$(PREFIX)/share/limine/'
	$(INSTALL) -m 644 BOOTRISCV64.EFI '$(DESTDIR)$(PREFIX)/share/limine/'
	$(INSTALL) -d '$(DESTDIR)$(PREFIX)/include'
	$(INSTALL) -m 644 limine.h '$(DESTDIR)$(PREFIX)/include/'
	$(INSTALL) -d '$(DESTDIR)$(PREFIX)/bin'
	$(INSTALL) limine '$(DESTDIR)$(PREFIX)/bin/'

.PHONY: install-strip
install-strip: install
	$(STRIP) '$(DESTDIR)$(PREFIX)/bin/limine'

.PHONY: clean
clean:
	rm -f limine limine.exe

limine: limine.c
	$(CC) $(CFLAGS) -Wall -Wextra $(WERROR_FLAG) $(CPPFLAGS) $(LDFLAGS) -std=c99 limine.c $(LIBS) -o $@

# ==========================================
# PHASE 20 AUTOMATION TARGETS
# ==========================================

release:
	@echo "[*] Compiling Kernel & Userland Payload..."
	./build_payload.sh
	@echo "[*] Building ISO..."
	@# If your ISO target is named differently, adjust the line below:
	$(MAKE) iso

run-sasos:
	@echo "[*] Booting SASOS Hardware Isolation Substrate..."
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-cpu max,+pku \
		-cdrom sexos-v1.0.0.iso \
		-serial stdio \
		-boot d


# ==========================================
# PHASE 20 AUTOMATION TARGETS
# ==========================================

release:
	@echo "[*] Compiling Kernel & Userland Payload..."
	./build_payload.sh
	@echo "[*] Building ISO..."
	@# If your ISO target is named differently, adjust the line below:
	$(MAKE) iso

run-sasos:
	@echo "[*] Booting SASOS Hardware Isolation Substrate..."
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-cpu max,+pku \
		-cdrom sexos-v1.0.0.iso \
		-serial stdio \
		-boot d

