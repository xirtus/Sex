#!/usr/bin/env bash
# INPUT_100_CURRENT_TIER_V1 gate.
# Usage: scripts/input_current_tier_gate.sh <serial-log>
# PASS requires every current-tier marker below plus a clean fault scan.
# Synthetic-only runs cannot pass: the usb.* upstream markers require real
# QEMU USB HID traffic (sexusb -> sexinput -> silk-shell).
set -u

LOG="${1:-}"
if [ -z "$LOG" ] || [ ! -f "$LOG" ]; then
    echo "usage: $0 <serial-log>"
    exit 2
fi

has() { grep -qE "$1" "$LOG"; }

PASS=1
row() {
    local name="$1" pattern="$2"
    if has "$pattern"; then
        echo "[gate.input.marker] $name PASS"
    else
        echo "[gate.input.marker] $name FAIL (missing: $pattern)"
        PASS=0
    fi
}

echo "== INPUT_100_CURRENT_TIER_V1 gate =="

# Upstream real-USB chain (rejects synthetic-only proof)
row usb_transfer_event       '\[sexusb\.hid\.transfer\.event\]'
row usb_rearm_ok             '\[sexusb\.hid\.rearm\.ok\]'
row usb_pointer_emit         '\[usb\.hid\.pointer\.emit\]'
row usb_pointer_shell_apply  '\[usb\.pointer\.shell\.apply\]'

# Behavior markers
row keyboard_keydown         '\[input\.keyboard\.keydown\.ok\]'
row keyboard_keyup           '\[input\.keyboard\.keyup\.ok\]'
row pointer_move             '\[input\.pointer\.move\.ok\]'
row button_down              '\[input\.button\.down\.ok\]'
row button_up                '\[input\.button\.up\.ok\]'
row click_hit_live           '\[silk\.click\.hit\.live\.ok\]'
row focus_set                '\[silk\.focus\.set\.ok\]'
row drag_begin               '\[silk\.drag\.begin\.ok\]'
row drag_move                '\[silk\.drag\.move\.ok\]'
row drag_end                 '\[silk\.drag\.end\.ok\]'

# Fault scan
pf=$(grep -c '#PF' "$LOG")
gp=$(grep -c '#GP' "$LOG")
panic=$(grep -ci 'panic' "$LOG")
fkill=$(grep -c 'fault\.kill' "$LOG")
reboot_loop=$(grep -Eci 'reboot[_ -]?loop|reset[_ -]?loop|triple fault' "$LOG")
freeze=$(grep -Eci 'freeze=1|frozen=1|freeze\.detected|watchdog\.freeze|scheduler\.freeze|input\.freeze|runtime\.freeze' "$LOG")
# IPC storm heuristic: pointer apply markers beyond sane interactive volume
applies=$(grep -c 'usb\.pointer\.shell\.apply' "$LOG")
storm=0
if [ "$applies" -gt 5000 ]; then storm=1; fi

if [ "$pf" -eq 0 ] && [ "$gp" -eq 0 ] && [ "$panic" -eq 0 ] && [ "$fkill" -eq 0 ] && [ "$reboot_loop" -eq 0 ] && [ "$freeze" -eq 0 ] && [ "$storm" -eq 0 ]; then
    echo "[input.faultscan.ok] pf=0 gp=0 panic=0 fault_kill=0 reboot_loop=0 freeze=0 storm=0"
else
    echo "[input.faultscan.fail] pf=$pf gp=$gp panic=$panic fault_kill=$fkill reboot_loop=$reboot_loop freeze=$freeze storm=$storm"
    PASS=0
fi

if [ "$PASS" -eq 1 ]; then
    echo "INPUT_100_CURRENT_TIER_V1: PASS"
    exit 0
else
    echo "INPUT_100_CURRENT_TIER_V1: FAIL"
    exit 1
fi
