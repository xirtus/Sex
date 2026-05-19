#!/usr/bin/env bash
# host_icmp_ping_observe_probe.sh — bounded host ping probe for Phase D
# Sends ping to SexOS guest at 10.0.2.15 via TAP interface.
# Reports PASS/FAIL/SKIP markers.
# No infinite loops, no password prompts.

set -euo pipefail

LOG="${1:-/tmp/sexnet_phase_d_host_ping.log}"
MAX_ATTEMPTS="${HOST_PING_MAX_ATTEMPTS:-3}"
TARGET_IP="${HOST_PING_TARGET:-10.0.2.15}"
TAP_IF="${HOST_PING_TAP_IF:-tap0}"
TIMEOUT="${HOST_PING_TIMEOUT:-2}"

exec > >(tee -a "$LOG") 2>&1

echo "[sexnet.phaseD.host_ping.begin] target=$TARGET_IP tap=$TAP_IF attempts=$MAX_ATTEMPTS"

# Check TAP interface
if ! ip link show "$TAP_IF" >/dev/null 2>&1; then
    echo "[sexnet.phaseD.host_ping.skip] reason=tap_interface_not_found if=$TAP_IF"
    exit 0
fi

# Check if ping works (CAP_NET_RAW or root)
PING_BIN="ping"
if ! command -v "$PING_BIN" >/dev/null 2>&1; then
    echo "[sexnet.phaseD.host_ping.skip] reason=ping_not_installed"
    exit 0
fi

# Quick privilege check: try sending one ping without waiting
if ! $PING_BIN -I "$TAP_IF" -c 1 -W 1 "$TARGET_IP" >/dev/null 2>&1; then
    # Check if it's a permission issue
    if $PING_BIN -I "$TAP_IF" -c 1 -W 1 "$TARGET_IP" 2>&1 | grep -qiE "permission|denied|socket|raw"; then
        echo "[sexnet.phaseD.host_ping.skip] reason=ping_requires_privilege"
        exit 0
    fi
fi

ATTEMPT=0
while [ "$ATTEMPT" -lt "$MAX_ATTEMPTS" ]; do
    ATTEMPT=$((ATTEMPT + 1))
    echo "[sexnet.phaseD.host_ping.ping] attempt=$ATTEMPT"

    PING_OUT="$($PING_BIN -I "$TAP_IF" -c 1 -W "$TIMEOUT" "$TARGET_IP" 2>&1)" || true

    if echo "$PING_OUT" | grep -qE "1 received|1 packets received|0% packet loss"; then
        echo "[sexnet.phaseD.host_ping.pass] attempt=$ATTEMPT target=$TARGET_IP"
        exit 0
    fi
done

echo "[sexnet.phaseD.host_ping.fail] attempts=$MAX_ATTEMPTS target=$TARGET_IP"
exit 0
