#!/usr/bin/env bash
# host_real_hw_nic_audit.sh — Phase N: Real Hardware NIC Model Audit
#
# Host-side, read-only, bounded probe of available NIC hardware.
# No root required. No destructive actions.
#
# Usage:
#   ./scripts/host_real_hw_nic_audit.sh [log_path]
#
#   log_path defaults to /tmp/sexnet_real_hw_nic_audit.log
#
# Returns:
#   0 — audit completed (supported or unsupported classification emitted)
#   1 — audit failed or no log writable

set -euo pipefail

LOG="${1:-/tmp/sexnet_real_hw_nic_audit.log}"
mkdir -p "$(dirname "$LOG")" 2>/dev/null || true
: > "$LOG" || { echo "FATAL: cannot write log: $LOG" >&2; exit 1; }

emit() { echo "$@" >> "$LOG"; echo "$@"; }

emit "============================================"
emit " SEXNET PHASE N: REAL HARDWARE NIC AUDIT"
emit "============================================"
emit "[host.real_hw.nic.audit.begin] date=$(date -Iseconds)"

# ── 1. lspci Ethernet / Network ──
WIRED_VENDOR=""
WIRED_DEVICE=""
WIFI_VENDOR=""
WIFI_DEVICE=""
NIC_CLASSIFICATION="UNKNOWN_ENVIRONMENT"

if command -v lspci &>/dev/null; then
    emit "[host.real_hw.nic.pci] lspci_available=1"

    while IFS= read -r line; do
        emit "[host.real_hw.nic.pci.line] $line"

        # Parse: "3d:00.0 Ethernet controller [0200]: Realtek ... [10ec:3000]"
        if echo "$line" | grep -qi "ethernet"; then
            WIRED_VENDOR=$(echo "$line" | grep -oP '\[\K[0-9a-fA-F]{4}(?=:[0-9a-fA-F]{4}\])' | head -1 || true)
            WIRED_DEVICE=$(echo "$line" | grep -oP '\[\K[0-9a-fA-F]{4}:[0-9a-fA-F]{4}\]' | head -1 | tr -d '[]' | cut -d: -f2 || true)
        fi
        if echo "$line" | grep -qi "network controller\|wireless\|wifi"; then
            WIFI_VENDOR=$(echo "$line" | grep -oP '\[\K[0-9a-fA-F]{4}(?=:[0-9a-fA-F]{4}\])' | head -1 || true)
            WIFI_DEVICE=$(echo "$line" | grep -oP '\[\K[0-9a-fA-F]{4}:[0-9a-fA-F]{4}\]' | head -1 | tr -d '[]' | cut -d: -f2 || true)
        fi
    done < <(lspci -nn 2>/dev/null | grep -iE 'ethernet|network' || true)
else
    emit "[host.real_hw.nic.pci] lspci_available=0"
fi

# ── 2. ip link ──
if command -v ip &>/dev/null; then
    emit "[host.real_hw.nic.ip_link] ip_available=1"
    while IFS= read -r line; do
        emit "[host.real_hw.nic.ip_link.line] $line"
    done < <(ip link show 2>/dev/null || true)
else
    emit "[host.real_hw.nic.ip_link] ip_available=0"
fi

# ── 3. sysfs vendor/device ──
WIRED_IFACE=""
WIFI_IFACE=""
for iface in /sys/class/net/*; do
    iface_name=$(basename "$iface")
    [ "$iface_name" = "lo" ] && continue
    driver_link=$(readlink "$iface/device/driver" 2>/dev/null || true)
    vendor=$(cat "$iface/device/vendor" 2>/dev/null || echo "N/A")
    device=$(cat "$iface/device/device" 2>/dev/null || echo "N/A")

    is_wired=0; is_wifi=0
    if [ -n "$driver_link" ]; then
        driver_name=$(basename "$driver_link" 2>/dev/null || true)
        emit "[host.real_hw.nic.sysfs] iface=$iface_name vendor=$vendor device=$device driver=$driver_name"
        case "$driver_name" in
            r8169|e1000|e1000e|igb|igc|tg3|bnx2|atlantic|r8125) is_wired=1 ;;
            iwlwifi|ath*|iwlegacy|rtl*_wifi|brcmfmac) is_wifi=1 ;;
        esac
    else
        emit "[host.real_hw.nic.sysfs] iface=$iface_name vendor=$vendor device=$device driver=unknown"
    fi

    if [ "$is_wired" -eq 1 ] && [ -z "$WIRED_IFACE" ]; then WIRED_IFACE="$iface_name"; fi
    if [ "$is_wifi" -eq 1 ] && [ -z "$WIFI_IFACE" ]; then WIFI_IFACE="$iface_name"; fi
done

# ── 4. ethtool (non-root, may fail) ──
if [ -n "${WIRED_IFACE:-}" ] && command -v ethtool &>/dev/null; then
    ethtool_info=$(ethtool -i "$WIRED_IFACE" 2>/dev/null || true)
    if [ -n "$ethtool_info" ]; then
        emit "[host.real_hw.nic.ethtool] iface=$WIRED_IFACE driver=$(echo "$ethtool_info" | grep '^driver:' | awk '{print $2}') version=$(echo "$ethtool_info" | grep '^version:' | awk '{print $2}')"
    fi
    link_info=$(ip link show "$WIRED_IFACE" 2>/dev/null | grep -o 'state [A-Z]*' | awk '{print $2}' || echo "UNKNOWN")
    emit "[host.real_hw.nic.link] iface=$WIRED_IFACE state=$link_info"
fi

# ── 5. Classification ──
WIRED_VENDOR_NUM=$((16#${WIRED_VENDOR:-"0"}))
WIRED_DEVICE_NUM=$((16#${WIRED_DEVICE:-"0"}))

# e1000/e1000e vendor = 0x8086 (Intel)
# e1000 device IDs: 0x100E, 0x100F, 0x1010, 0x1012, 0x1013, 0x1015, 0x1016, 0x1017, 0x1018, 0x1019, 0x101A, 0x101D, 0x101E, 0x1026, 0x1027, 0x1028, 0x1049, 0x104A, 0x104B, 0x104C, 0x104D, 0x105E, 0x105F, 0x1060, 0x1075, 0x1076, 0x1077, 0x1078, 0x1079, 0x107A, 0x107B, 0x107C, 0x107D, 0x107E, 0x107F, 0x108A, 0x108B, 0x108C, 0x1096, 0x1098, 0x1099, 0x109A, 0x10A4, 0x10A5, 0x10B5, 0x10B9, 0x10BA, 0x10BB, 0x10BC, 0x10C4, 0x10C5, 0x10C9, 0x10D3, 0x10D5, 0x10D6, 0x10D9, 0x10DA, 0x10E5, 0x10E6, 0x10E7, 0x10E8, 0x10EA, 0x10EB, 0x10EC, 0x10EF, 0x10F0, 0x10F1, 0x10F4, 0x10F5, 0x10F6, 0x10F7, 0x10F8, 0x10F9, 0x10FA, 0x10FB, 0x10FC, 0x1501, 0x1502, 0x1503, 0x1507, 0x150C, 0x1521, 0x1522, 0x1523, 0x1524, 0x1525, 0x1526, 0x1527, 0x1528, 0x1533, 0x1536, 0x1537, 0x1538, 0x1539, 0x153A, 0x153B, 0x1559, 0x155A, 0x156F, 0x1570, 0x157B, 0x157C, 0x15A0, 0x15A1, 0x15A2, 0x15A3, 0x15B7, 0x15B8, 0x15B9, 0x15BB, 0x15BC, 0x15BD, 0x15BE, 0x15DF, 0x15E0, 0x15E1, 0x15E2, 0x15E3, 0x1F40, 0x1F41, 0x1F45, 0x3100, 0x3101, 0x5502, 0x5504

is_intel_e1000_vendor=0
[ "$WIRED_VENDOR_NUM" -eq 32902 ] && is_intel_e1000_vendor=1  # 0x8086 = 32902

# Check if device is known e1000/e1000e
E1000_DEV_IDS="100E 100F 1010 1012 1013 1015 1016 1017 1018 1019 101A 101D 101E 1026 1027 1028 1049 104A 104B 104C 104D 105E 105F 1060 1075 1076 1077 1078 1079 107A 107B 107C 107D 107E 107F 108A 108B 108C 1096 1098 1099 109A 10A4 10A5 10B5 10B9 10BA 10BB 10BC 10C4 10C5 10C9 10D3 10D5 10D6 10D9 10DA 10E5 10E6 10E7 10E8 10EA 10EB 10EC 10EF 10F0 10F1 10F4 10F5 10F6 10F7 10F8 10F9 10FA 10FB 10FC 1501 1502 1503 1507 150C 1521 1522 1523 1524 1525 1526 1527 1528 1533 1536 1537 1538 1539 153A 153B 1559 155A 156F 1570 157B 157C 15A0 15A1 15A2 15A3 15B7 15B8 15B9 15BB 15BC 15BD 15BE 15DF 15E0 15E1 15E2 15E3 1F40 1F41 1F45 3100 3101 5502 5504"
is_e1000_device=0
for did in $E1000_DEV_IDS; do
    if [ "$WIRED_DEVICE" = "$did" ]; then
        is_e1000_device=1
        break
    fi
done

SUPPORT=0
REASON=""

if [ -z "$WIRED_VENDOR" ] || [ -z "$WIRED_DEVICE" ]; then
    NIC_CLASSIFICATION="NO_WIRED_NIC_FOUND"
    REASON="no_wired_ethernet_nic_detected"
elif [ "$is_intel_e1000_vendor" -eq 1 ] && [ "$is_e1000_device" -eq 1 ]; then
    NIC_CLASSIFICATION="SUPPORTED_E1000_COMPATIBLE"
    SUPPORT=1
    REASON="intel_e1000_compatible_nic_found"
elif [ "$is_intel_e1000_vendor" -eq 1 ] && [ "$is_e1000_device" -eq 0 ]; then
    # Intel but not e1000 — might be I219 or similar, needs e1000e audit
    NIC_CLASSIFICATION="POSSIBLY_E1000E_REQUIRES_AUDIT"
    REASON="intel_nic_not_in_e1000_list_may_be_e1000e"
else
    NIC_CLASSIFICATION="UNSUPPORTED_MODERN_NIC"
    REASON="vendor=${WIRED_VENDOR}_device=${WIRED_DEVICE}_not_e1000_compatible"
fi

emit ""
emit "── Real Hardware NIC Classification ──"
emit "[host.real_hw.nic.wired] vendor=0x${WIRED_VENDOR:-N/A} device=0x${WIRED_DEVICE:-N/A} iface=${WIRED_IFACE:-none}"
emit "[host.real_hw.nic.wifi] vendor=0x${WIFI_VENDOR:-N/A} device=0x${WIFI_DEVICE:-N/A} iface=${WIFI_IFACE:-none} supported_for_sexnet=0 reason=ethernet_only"
emit "[host.real_hw.nic.support] supported=${SUPPORT} classification=${NIC_CLASSIFICATION} reason=${REASON}"

# ── 6. Recommended next action ──
emit "[host.real_hw.nic.recommendation]"
if [ "$NIC_CLASSIFICATION" = "SUPPORTED_E1000_COMPATIBLE" ]; then
    emit "  action=PROCEED_TO_BAR_MAP_PROOF"
    emit "  note=Real hardware supports e1000-compatible NIC. QEMU regression must remain PASS."
elif [ "$NIC_CLASSIFICATION" = "POSSIBLY_E1000E_REQUIRES_AUDIT" ]; then
    emit "  action=STOP_FIRST_NEEDS_E1000E_DRIVER_AUDIT"
    emit "  note=Intel NIC found but not in confirmed e1000 list. Do not write MMIO registers."
elif [ "$NIC_CLASSIFICATION" = "UNSUPPORTED_MODERN_NIC" ]; then
    emit "  action=SKIP_REAL_HARDWARE_PROOFS_STAY_WITH_QEMU_SOURCE3"
    emit "  note=Real NIC is modern/non-e1000. SexNet real hardware BAR/RX/TX/ARP/PING are ALL SKIP. QEMU source3 remains primary proven path."
elif [ "$NIC_CLASSIFICATION" = "NO_WIRED_NIC_FOUND" ]; then
    emit "  action=SKIP_ALL_REAL_HARDWARE_PROOFS"
    emit "  note=No wired Ethernet NIC found. No real hardware networking possible."
else
    emit "  action=SKIP_REAL_HARDWARE_PROOFS_INSUFFICIENT_INFO"
    emit "  note=Insufficient host environment information. Stay with QEMU."
fi

emit ""
emit "[sexnet.real_hw.nic_model.audit.done] classification=${NIC_CLASSIFICATION} ok=1"
emit "[host.real_hw.nic.audit.done] ok=1"

exit 0
