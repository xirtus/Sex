#!/bin/bash
pkill -9 -f qemu-system-x86_64 || true
rm -f /tmp/sexos-qmp.sock
exec /home/xirtus_arch/Documents/microkernel/tools/qemu/build/qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku -cdrom sexos-debug.iso -device nec-usb-xhci,id=xhci -device usb-tablet,bus=xhci.0 -serial stdio -display none -qmp unix:/tmp/sexos-qmp.sock,server=on,wait=off > qemu_debug.log 2>&1
