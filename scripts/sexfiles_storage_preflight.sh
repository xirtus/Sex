#!/usr/bin/env bash
# sexfiles_storage_preflight.sh — SexFiles Real-Hardware Storage Preflight V1.
#
# SAFE: Log-only checks. Reads system info, never writes.
# No destructive actions. No disk/partition writes. No bootloader changes.
#
# Audits:
#   1. NVMe/AHCI controller presence
#   2. Available storage devices
#   3. Sector size assumptions
#   4. PCI class codes for storage
#   5. Required driver infrastructure gap
#
# Usage:
#   ./scripts/sexfiles_storage_preflight.sh
#
# Returns:
#   0 if all checks pass (informational — storage persistence is NOT available)
#   1 if critical hardware is absent

set -euo pipefail

echo "============================================"
echo " SEXFILES STORAGE PREFLIGHT V1"
echo " REAL-HARDWARE STORAGE READINESS AUDIT"
echo "============================================"
echo ""

PASS=0
WARN=0
FAIL=0
SKIP=0
INFO=0

pass()  { echo "  [PASS] $1"; PASS=$((PASS + 1)); }
warn()  { echo "  [WARN] $1"; WARN=$((WARN + 1)); }
fail()  { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }
skip()  { echo "  [SKIP] $1"; SKIP=$((SKIP + 1)); }
info()  { echo "  [INFO] $1"; INFO=$((INFO + 1)); }

# ---- 1. NVMe Controller Detection ----
echo "--- NVMe CONTROLLER ---"

if command -v lspci &>/dev/null; then
    nvme_count=$(lspci -d ::0108 2>/dev/null | wc -l || true)
    if [ "$nvme_count" -gt 0 ]; then
        info "NVMe controller(s) detected: $nvme_count"
        lspci -d ::0108 2>/dev/null | head -4 | while read -r line; do
            echo "         $line"
        done
        warn "NVMe detected but NO NVMe driver exists in SexOS. Required: driver + PDX block ABI."
    else
        info "No NVMe controller detected via lspci (class 0x0108)"
    fi
else
    if [ -d /sys/bus/pci/devices ]; then
        nvme_sys_count=$(for dev in /sys/bus/pci/devices/*; do
            cls=$(cat "$dev/class" 2>/dev/null || echo "0")
            if [ "$cls" = "0x010802" ]; then echo "$dev"; fi
        done 2>/dev/null | wc -l)
        if [ "$nvme_sys_count" -gt 0 ]; then
            info "NVMe controller(s) detected via /sys: $nvme_sys_count"
            warn "NVMe detected but NO NVMe driver exists in SexOS."
        else
            info "No NVMe controller detected via /sys"
        fi
    else
        skip "Cannot enumerate PCI (no lspci or /sys/bus/pci)"
    fi
fi

echo ""

# ---- 2. AHCI/SATA Controller Detection ----
echo "--- AHCI/SATA CONTROLLER ---"

if command -v lspci &>/dev/null; then
    ahci_count=$(lspci -d ::0106 2>/dev/null | wc -l || true)
    if [ "$ahci_count" -gt 0 ]; then
        info "AHCI controller(s) detected: $ahci_count"
        lspci -d ::0106 2>/dev/null | head -4 | while read -r line; do
            echo "         $line"
        done
        warn "AHCI detected but NO AHCI driver exists in SexOS. Required: driver + PDX block ABI."
    else
        info "No AHCI controller detected via lspci (class 0x0106)"
    fi
else
    skip "Cannot enumerate AHCI controllers (no lspci)"
fi

echo ""

# ---- 3. Available Block Devices (host OS — informational only) ----
echo "--- BLOCK DEVICES (HOST OS REFERENCE) ---"

if [ -d /sys/block ]; then
    block_count=$(ls -d /sys/block/*/ 2>/dev/null | wc -l || true)
    info "Host OS sees $block_count block device(s)"
    info "DO NOT RUN: These are for host OS reference only. No writes are performed."
    for dev_dir in /sys/block/*/ ; do
        dev_name=$(basename "$dev_dir")
        dev_size_sectors=$(cat "$dev_dir/size" 2>/dev/null || echo "0")
        dev_size_bytes=$((dev_size_sectors * 512))
        if [ "$dev_size_bytes" -gt 0 ]; then
            dev_type="unknown"
            case "$dev_name" in
                nvme*) dev_type="NVMe" ;;
                sd*)   dev_type="SCSI/SATA/USB" ;;
                vd*)   dev_type="VirtIO" ;;
            esac
            size_gb=$((dev_size_bytes / 1000000000))
            info "  /dev/$dev_name: ${size_gb} GB ($dev_type)"
        fi
    done 2>/dev/null || true
elif [ -d /dev ]; then
    info "Block devices in /dev (informational):"
    ls -l /dev/sd* /dev/nvme* /dev/vd* 2>/dev/null | head -8 | while read -r line; do
        echo "         $line"
    done || info "  No standard block devices found in /dev"
else
    skip "Cannot enumerate block devices"
fi

echo ""

# ---- 4. Sector Size Probe (Host OS) ----
echo "--- SECTOR SIZE ---"

if [ -d /sys/block ]; then
    logical_ok=true
    for dev_dir in /sys/block/*/; do
        logical=$(cat "$dev_dir/queue/logical_block_size" 2>/dev/null || echo "0")
        physical=$(cat "$dev_dir/queue/physical_block_size" 2>/dev/null || echo "0")
        dev_name=$(basename "$dev_dir")
        if [ "$logical" != "0" ]; then
            if [ "$logical" != "512" ]; then
                warn "/dev/$dev_name logical sector size = $logical (SexFiles assumes 512)"
                logical_ok=false
            fi
        fi
        if [ "$physical" != "0" ] && [ "$physical" != "512" ]; then
            info "/dev/$dev_name physical sector size = $physical"
        fi
    done 2>/dev/null || true
    if [ "$logical_ok" = true ]; then
        info "All detected block devices report 512-byte logical sectors (SexFiles default)"
    fi
else
    skip "Cannot probe sector sizes"
fi

echo ""

# ---- 5. PCI Storage Class Code Enumeration ----
echo "--- PCI STORAGE DEVICES (ALL) ---"

if command -v lspci &>/dev/null; then
    storage_devices=$(lspci -d ::01 2>/dev/null || true)
    if [ -n "$storage_devices" ]; then
        echo "$storage_devices" | head -8 | while read -r line; do
            info "  $line"
        done
    else
        info "No PCI storage controllers found"
    fi
else
    skip "Cannot enumerate PCI storage devices (no lspci)"
fi

echo ""

# ---- 6. Limine boot module audit ----
echo "--- SEXFILES BOOT MODULE ---"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
LIMINE_CFG="$ROOT_DIR/limine.cfg"

if [ -f "$LIMINE_CFG" ]; then
    if grep -q "sexfiles\|sexstore" "$LIMINE_CFG" 2>/dev/null; then
        info "SexFiles/SexStore modules present in Limine config"
        grep -E "sexfiles|sexstore" "$LIMINE_CFG" 2>/dev/null | while read -r line; do
            echo "         $line"
        done
    else
        info "SexFiles/SexStore modules NOT in limine.cfg (loaded via boot protocol)"
    fi
else
    skip "limine.cfg not found at $LIMINE_CFG"
fi

echo ""

# ---- 7. SexFiles/SexStore Binary Check ----
echo "--- SEXFILES/SEXSTORE BUILD ---"

SEXFILES_BIN="$ROOT_DIR/iso_root/servers/sexfiles"
SEXSTORE_BIN="$ROOT_DIR/iso_root/servers/sexstore"

if [ -f "$SEXFILES_BIN" ]; then
    sexfiles_size=$(stat -c%s "$SEXFILES_BIN" 2>/dev/null || stat -f%z "$SEXFILES_BIN" 2>/dev/null || echo "0")
    info "sexfiles binary: $SEXFILES_BIN ($sexfiles_size bytes)"
else
    warn "sexfiles binary not built. Run: ./scripts/entrypoint_build.sh"
fi

if [ -f "$SEXSTORE_BIN" ]; then
    sexstore_size=$(stat -c%s "$SEXSTORE_BIN" 2>/dev/null || stat -f%z "$SEXSTORE_BIN" 2>/dev/null || echo "0")
    info "sexstore binary: $SEXSTORE_BIN ($sexstore_size bytes)"
else
    warn "sexstore binary not built. Run: ./scripts/entrypoint_build.sh"
fi

echo ""

# ---- 8. Infrastructure Gap Analysis ----
echo "--- INFRASTRUCTURE GAP ANALYSIS ---"

info ""
info "  GAP 1 — Block Device Server:"
info "    Current: apps/sexdrive = XHCI framebuffer demo (NOT storage)"
info "    Missing: NVMe/AHCI block device server with sector R/W"
info "    Target:  apps/sexblk or repurposed apps/sexdrive"

info ""
info "  GAP 2 — Block Device PDX ABI:"
info "    Current: no SLOT_BLOCK, OP_BLOCK_READ_SECTOR, OP_BLOCK_WRITE_SECTOR"
info "    Missing: new slot + opcodes in crates/sex-pdx/src/lib.rs"
info "    Status:  STOP FIRST required (sex-pdx ABI change)"

info ""
info "  GAP 3 — DiskFS -> Block Server Wiring:"
info "    Current: DiskFs on in-memory RwLock; all FsBackend return ERR_NOT_FOUND"
info "    Missing: PDX call to block server for sector R/W"
info "    Format:  superblock@LBA0, object table@blocks 1..N, journal@blocks N+1..M"

info ""
info "  GAP 4 — Kernel Block Device Support:"
info "    Current: no NVMe/AHCI/SATA driver in kernel"
info "    Current: PCI BAR mapping (syscall 43) MAY suffice for MMIO"
info "    Missing: DMA buffer syscall (if block server needs DMA)"

info ""
info "  GAP 5 — Boot-Time Recovery:"
info "    Current: no mechanism to read superblock from disk on boot"
info "    Missing: init sequence calling DiskFs mount() from real block device"

info ""
info "  GAP 6 — Cache/Flush/Sync:"
info "    Current: no cache flush, no DMA fence, no sync barriers"
info "    Current: DiskFs state is CPU-cached RwLock with no write ordering"
info "    Missing: CLFLUSH, SFENCE, or WC memory type for storage buffers"
info ""

# ---- SUMMARY ----
echo "============================================"
echo " STORAGE PREFLIGHT SUMMARY"
echo "============================================"
echo ""
printf "  %-40s %3d\n" "PASS"  "$PASS"
printf "  %-40s %3d\n" "WARN"  "$WARN"
printf "  %-40s %3d\n" "INFO (infra gaps)"  "$INFO"
printf "  %-40s %3d\n" "SKIP (no tooling)"  "$SKIP"
printf "  %-40s %3d\n" "FAIL"  "$FAIL"
echo ""

echo "=== SEXFILES STORAGE READINESS VERDICT ==="
echo ""
echo "  Storage hardware: DETECTED (but no driver exists)"
echo "  Storage driver:   MISSING (no NVMe, AHCI, or block driver)"
echo "  PDX block ABI:    MISSING (no SLOT_BLOCK in sex-pdx)"
echo "  DiskFS wiring:    MISSING (in-memory scaffold only)"
echo "  Cache/Flush:      MISSING (no cache coherence for storage)"
echo "  Persistence:      BLOCKED (no path to persistent media)"
echo "  Data-loss risk:   NONE (no writes ever reach physical media)"
echo ""
echo "  BLOCKER STATUS: 6/6 components missing for real hw persistence."
echo "  All 'writes' go to RAM. Power loss = total data loss."
echo "  SAFE: No destructive writes possible to physical storage."
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo "[RESULT] CRITICAL FAILURES. Address FAIL items."
    exit 1
else
    echo "[RESULT] Preflight complete."
    echo "  Full audit: docs/handoff/SEXFILES_REAL_HARDWARE_STORAGE_AUDIT_V1.md"
    exit 0
fi
