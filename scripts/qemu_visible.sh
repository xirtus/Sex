#!/usr/bin/env bash
set -euo pipefail

ISO="${1:-sexos-v1.0.0.iso}"

qemu-system-x86_64 \
  -M q35 \
  -m 512M \
  -cpu max,+pku \
  -cdrom "$ISO" \
  -device nec-usb-xhci,id=xhci \
  -device usb-tablet,bus=xhci.0 \
  -serial stdio \
  -display gtk \
  -boot d
