#!/usr/bin/env bash

# Exit immediately if a command fails
set -e

echo "[*] INITIATING FINAL BOOT SEQUENCE"

# Surgically replace the dummy ISO name with the real one in the Makefile.
# Note: macOS uses BSD sed, which requires the empty string '' after -i.
sed -i '' 's/-cdrom sexos.iso/-cdrom sexos-v1.0.0.iso/g' Makefile

echo "[*] Makefile successfully patched with actual ISO name."
echo "[*] IGNITION: Booting SASOS Hardware Isolation Substrate..."

# Execute the QEMU boot target directly (since the build is already complete)
make run-sasos

