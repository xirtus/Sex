#!/usr/bin/env bash
# real_hardware_preflight.sh — SexOS Real-Hardware Boot Preflight Checklist V1.
#
# SAFE: Log-only checks. No kernel edits, no bootloader changes, no structural changes.
# Run on the TARGET machine BEFORE attempting to boot SexOS from USB/CD.
#
# Usage:
#   ./scripts/real_hardware_preflight.sh
#
# Returns:
#   0 if all preflight checks PASS or WARN
#   1 if any CRITICAL check fails

set -euo pipefail

echo "============================================"
echo " SEXOS REAL-HARDWARE PREFLIGHT V1"
echo "============================================"
echo ""

PASS=0
WARN=0
FAIL=0
SKIP=0

pass()  { echo "  [PASS] $1"; PASS=$((PASS + 1)); }
warn()  { echo "  [WARN] $1"; WARN=$((WARN + 1)); }
fail()  { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }
skip()  { echo "  [SKIP] $1"; SKIP=$((SKIP + 1)); }

# ---- 1. CPU Features ----
echo "--- CPU FEATURES ---"

# PKU check via /proc/cpuinfo
if [ -f /proc/cpuinfo ]; then
    if grep -q " pku " /proc/cpuinfo 2>/dev/null; then
        pass "PKU feature flag present in /proc/cpuinfo"
    else
        warn "PKU not detected in /proc/cpuinfo. SexOS will run without MPK isolation."
    fi
else
    skip "Cannot read /proc/cpuinfo (not Linux or no /proc)"
fi

# x86-64 check
if [ "$(uname -m 2>/dev/null)" = "x86_64" ]; then
    pass "x86-64 architecture confirmed"
else
    arch_val=$(uname -m 2>/dev/null || echo "unknown")
    warn "Architecture is '$arch_val' (expected x86_64). SexOS requires x86-64."
fi

# ---- 2. Serial Port ----
echo ""
echo "--- SERIAL PORT ---"

if [ -d /sys/class/tty/ttyS0 ]; then
    pass "Serial port ttyS0 (COM1 at 0x3F8) detected by kernel"
    
    # Check if it's usable
    if [ -w /sys/class/tty/ttyS0/device/resources ] 2>/dev/null; then
        io_range=$(cat /sys/class/tty/ttyS0/port 2>/dev/null || echo "unknown")
        pass "ttyS0 I/O range: $io_range"
    fi
else
    warn "ttyS0 not detected. Connect USB-serial adapter or verify firmware serial settings."
    warn "SexOS hardcodes COM1 (0x3F8). If serial is absent, kernel may panic on debug output."
fi

# Check serial-USB adapter
for dev in /dev/ttyUSB*; do
    if [ -c "$dev" 2>/dev/null ]; then
        pass "USB-serial adapter found: $dev"
        break
    fi
done 2>/dev/null || true

# ---- 3. Memory ----
echo ""
echo "--- MEMORY ---"

total_ram_kb=$(grep MemTotal /proc/meminfo 2>/dev/null | awk '{print $2}' || echo 0)
if [ "$total_ram_kb" -gt 524288 ]; then
    pass "Sufficient RAM: $((total_ram_kb / 1024)) MB (minimum 512 MB required)"
else
    warn "Low RAM: $((total_ram_kb / 1024)) MB (minimum 512 MB for SexOS)"
fi

# ---- 4. UEFI/BIOS Mode ----
echo ""
echo "--- FIRMWARE ---"

if [ -d /sys/firmware/efi ]; then
    pass "Booted in UEFI mode (Limine supports UEFI)"
else
    warn "Booted in legacy BIOS/CSM mode (Limine supports both, but verify boot method)"
fi

if [ -d /sys/firmware/efi ] && command -v mokutil &>/dev/null; then
    if mokutil --sb-state 2>/dev/null | grep -qi "enabled"; then
        warn "Secure Boot is ENABLED. SexOS does not support Secure Boot. Disable in firmware."
    else
        pass "Secure Boot is disabled"
    fi
fi

# ---- 5. IOMMU / VT-d / AMD-Vi ----
echo ""
echo "--- VIRTUALIZATION ---"

if grep -q "iommu" /proc/cmdline 2>/dev/null; then
    warn "IOMMU enabled. SexOS does not use IOMMU; pass-through not configured."
fi

if grep -q " svm " /proc/cpuinfo 2>/dev/null; then
    pass "AMD SVM (IOMMU) available"
elif grep -q " vmx " /proc/cpuinfo 2>/dev/null; then
    pass "Intel VT-x (IOMMU) available"
else
    skip "Virtualization not detected (not required for SexOS)"
fi

# ---- 6. USB Controller ----
echo ""
echo "--- USB ---"

# Look for XHCI controller in lspci or /sys
if command -v lspci &>/dev/null; then
    xhci_count=$(lspci -d ::0c03 2>/dev/null | grep -i xhci | wc -l || true)
    if [ "$xhci_count" -gt 0 ]; then
        pass "XHCI USB controller(s) detected: $xhci_count"
        lspci -d ::0c03 2>/dev/null | grep -i xhci | head -3 | while read -r line; do
            echo "         $line"
        done
    else
        warn "No XHCI controller detected. SexOS requires USB XHCI for USB input."
    fi
elif [ -d /sys/bus/pci ]; then
    xhci_count=$(find /sys/bus/pci/devices -name "class" -exec grep -l 0c0330 {} \; 2>/dev/null | wc -l)
    if [ "$xhci_count" -gt 0 ]; then
        pass "XHCI controller(s) detected: $xhci_count"
    else
        warn "No XHCI controller found via /sys (ProgIF 0x30)."
    fi
else
    skip "Cannot enumerate USB controllers (no lspci or /sys/bus/pci)"
fi

# ---- 7. Boot Device ----
echo ""
echo "--- ISO / BOOT MEDIUM ---"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ISO="$ROOT_DIR/sexos-v1.0.0.iso"

if [ -f "$ISO" ]; then
    iso_size=$(stat -c%s "$ISO" 2>/dev/null || stat -f%z "$ISO" 2>/dev/null || echo 0)
    if [ "$iso_size" -gt 1048576 ]; then
        pass "ISO exists: $ISO ($((iso_size / 1048576)) MB)"
    else
        warn "ISO is very small ($iso_size bytes). May be incomplete."
    fi

    # Check if limine bios-install has been run (search for Limine MBR signature)
    # Limine BIOS install writes signature bytes at specific offsets.
    # This is a soft check — we look for the string "LIMINE" in the ISO.
    if strings "$ISO" 2>/dev/null | grep -q "LIMINE" 2>/dev/null || \
       dd if="$ISO" bs=1 skip=3 count=6 2>/dev/null | grep -q "LIMINE" 2>/dev/null; then
        pass "ISO contains Limine boot signature"
    else
        warn "ISO may be missing 'limine bios-install'. Run: ./limine/limine bios-install sexos-v1.0.0.iso"
        warn "Real hardware BIOS boot WILL FAIL without this step."
    fi
else
    fail "ISO not found: $ISO. Run: ./scripts/entrypoint_build.sh"
fi

# ---- 8. Limine Tool ----
echo ""
echo "--- LIMINE TOOL ---"

LIMINE_BIN="$ROOT_DIR/limine/limine"
if [ -f "$LIMINE_BIN" ] && [ -x "$LIMINE_BIN" ]; then
    pass "Limine tool found: $LIMINE_BIN"
    limine_ver=$("$LIMINE_BIN" --version 2>/dev/null || echo "version unknown")
    echo "         $limine_ver"
else
    fail "Limine tool not found at $LIMINE_BIN. Run: ./scripts/bootstrap_limine.sh"
fi

# ---- 9. UEFI Boot Files ----
echo ""
echo "--- UEFI BOOT ---"

EFI_FILE="$ROOT_DIR/limine/BOOTX64.EFI"
if [ -f "$EFI_FILE" ]; then
    pass "UEFI boot file found: $EFI_FILE"
else
    warn "UEFI boot file not found. UEFI boot may fail."
fi

# ---- 10. Cargo Target ----
echo ""
echo "--- BUILD TARGET ---"

TARGET_JSON="$ROOT_DIR/../x86_64-sex.json"  # relative to ROOT_DIR
TARGET_JSON2="$HOME/x86_64-sex.json"
if [ -f "$TARGET_JSON" ]; then
    pass "Target spec found: $TARGET_JSON"
elif [ -f "$TARGET_JSON2" ]; then
    pass "Target spec found: $TARGET_JSON2"
else
    # Check in common locations
    TARGET_JSON3=$(find / -name "x86_64-sex.json" -maxdepth 3 2>/dev/null | head -1 || true)
    if [ -n "$TARGET_JSON3" ]; then
        pass "Target spec found: $TARGET_JSON3"
    else
        fail "Target spec x86_64-sex.json not found. Build will fail."
    fi
fi

# ---- SUMMARY ----
echo ""
echo "============================================"
echo " PREFLIGHT SUMMARY"
echo "============================================"
echo ""
printf "  %-30s %3d\n" "PASS"  "$PASS"
printf "  %-30s %3d\n" "WARN"  "$WARN"
printf "  %-30s %3d\n" "FAIL"  "$FAIL"
printf "  %-30s %3d\n" "SKIP"  "$SKIP"
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo "[RESULT] CRITICAL FAILURES DETECTED. Review FAIL items before boot."
    exit 1
elif [ "$WARN" -gt 4 ]; then
    echo "[RESULT] Many warnings. Review recommended before hardware boot."
    exit 0
else
    echo "[RESULT] Preflight looks OK. Ensure warnings are acceptable for your test scenario."
    exit 0
fi
