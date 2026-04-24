iso:
	rm -f sexos-v1.0.0.iso
	xorriso -as mkisofs -R -r -J \
	  -b boot/limine/limine-bios-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table \
	  --efi-boot boot/limine/limine-uefi-cd.bin -efi-boot-part --efi-boot-image --protective-msdos-label \
	  iso_root -o sexos-v1.0.0.iso
run-sasos:
	qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku -cdrom sexos-v1.0.0.iso -serial stdio -boot d -qmp unix:qmp.sock,server,nowait
build-tools:
	cd tools/sex-debug && cargo build --release
