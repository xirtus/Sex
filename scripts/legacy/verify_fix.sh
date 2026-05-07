#!/bin/bash
set -e
QEMU_BIN="/home/xirtus_arch/Documents/microkernel/tools/qemu/build/qemu-system-x86_64"
ISO="/home/xirtus_arch/Documents/microkernel/sexos-debug.iso"
QMP_SOCK="/tmp/sexos-qmp.sock"
LOG="/home/xirtus_arch/Documents/microkernel/verify_final.log"

pkill -9 -f qemu-system-x86_64 || true
rm -f $QMP_SOCK $LOG

echo "Starting QEMU..."
stdbuf -oL -eL $QEMU_BIN -M q35 -m 512M -cpu max,+pku -cdrom $ISO -device nec-usb-xhci,id=xhci -device usb-tablet,bus=xhci.0 -serial stdio -display none -qmp unix:$QMP_SOCK,server=on,wait=off > $LOG 2>&1 &
QEMU_PID=$!

echo "Waiting for boot (180s)..."
sleep 180

echo "Injecting click..."
python3 /home/xirtus_arch/Documents/microkernel/scripts/qemu_mouse_inject.py --click

echo "Waiting for guest processing (60s)..."
sleep 60

echo "Checking logs..."
if grep -a "\[sexinput\] click" $LOG; then
    echo "SUCCESS: Click detected in guest log."
else
    echo "SUMMARY (Last 100 QEMU TRACE):"
    grep -a "QEMU_TRACE" $LOG | tail -n 100
    echo "SUMMARY (Last 100 GUEST LOG):"
    tail -n 100 $LOG | grep -v "QEMU_TRACE"
fi

kill $QEMU_PID || true
