#!/usr/bin/env bash
set -e

CMD="${1:-run}"
TRACE_ARGS=""
if [ -n "$SEXUSB_XHCI_TRACE" ]; then
    TRACE_ARGS="-trace usb_xhci_slot_address -trace usb_xhci_queue_event -trace usb_xhci_fetch_trb -trace usb_xhci_doorbell"
fi

# Select USB device: mouse (boot HID, default) or tablet (absolute, no boot HID).
USB_DEV="${SEXUSB_QEMU_DEVICE:-mouse}"
case "$USB_DEV" in
  mouse)  USB_DEVICE_ARG="-device usb-mouse,bus=xhci.0" ;;
  tablet) USB_DEVICE_ARG="-device usb-tablet,bus=xhci.0" ;;
  *)
    echo "error: unknown SEXUSB_QEMU_DEVICE=$USB_DEV (use mouse or tablet)"
    exit 1
    ;;
esac

case "$CMD" in
  run)
    qemu-system-x86_64 \
      -M q35 \
      -m 512M \
      -cpu max,+pku \
      -cdrom sexos-v1.0.0.iso \
      -device nec-usb-xhci,id=xhci \
      $USB_DEVICE_ARG \
      $TRACE_ARGS \
      -serial stdio \
      -display sdl
    ;;
  run-nographic)
    qemu-system-x86_64 \
      -M q35 \
      -m 512M \
      -cpu max,+pku \
      -cdrom sexos-v1.0.0.iso \
      -device nec-usb-xhci,id=xhci \
      $USB_DEVICE_ARG \
      $TRACE_ARGS \
      -display none \
      -serial stdio
    ;;
  *)
    echo "usage: ./dev.sh [run|run-nographic]"
    exit 1
    ;;
esac
