#!/usr/bin/env bash
# host_udp_echo_observe_probe.sh -- bounded host UDP echo probe for Phase E
# Sends UDP payload to SexOS guest at 10.0.2.15 via TAP interface.
# Expects an echo reply from sexnet UDP echo handler.
# Reports PASS/FAIL/SKIP markers.
# No infinite loops, no password prompts.

set -euo pipefail

LOG="${1:-/tmp/sexnet_phase_e_host_udp.log}"
MAX_ATTEMPTS="${HOST_UDP_MAX_ATTEMPTS:-3}"
TARGET_IP="${HOST_UDP_TARGET:-10.0.2.15}"
TARGET_PORT="${HOST_UDP_PORT:-7777}"
PAYLOAD="${HOST_UDP_PAYLOAD:-HELLO_SEXNET_UDP_ECHO}"
TIMEOUT="${HOST_UDP_TIMEOUT:-2}"
TAP_IF="${HOST_UDP_TAP_IF:-tap0}"

exec > >(tee -a "$LOG") 2>&1

echo "[sexnet.phaseE.host_udp.begin] target=$TARGET_IP port=$TARGET_PORT tap=$TAP_IF attempts=$MAX_ATTEMPTS"

# Check TAP interface
if ! ip link show "$TAP_IF" >/dev/null 2>&1; then
    echo "[sexnet.phaseE.host_udp.skip] reason=tap_interface_not_found if=$TAP_IF"
    exit 0
fi

# Check if nc is available
if ! command -v nc >/dev/null 2>&1; then
    echo "[sexnet.phaseE.host_udp.skip] reason=nc_not_installed"
    exit 0
fi

ATTEMPT=0
while [ "$ATTEMPT" -lt "$MAX_ATTEMPTS" ]; do
    ATTEMPT=$((ATTEMPT + 1))
    echo "[sexnet.phaseE.host_udp.send] attempt=$ATTEMPT payload=$PAYLOAD target=$TARGET_IP:$TARGET_PORT"

    # Send UDP payload and wait for reply with timeout
    # nc -u sends to UDP, -w sets timeout for the whole connection
    REPLY=$(echo -n "$PAYLOAD" | nc -u -w "$TIMEOUT" "$TARGET_IP" "$TARGET_PORT" 2>&1) || true

    if [ -n "$REPLY" ] && echo "$REPLY" | grep -q "$PAYLOAD"; then
        echo "[sexnet.phaseE.host_udp.pass] attempt=$ATTEMPT target=$TARGET_IP:$TARGET_PORT reply_seen=1"
        exit 0
    fi

    # If no reply but nc didn't error, check if we got anything at all
    if [ -n "$REPLY" ]; then
        echo "[sexnet.phaseE.host_udp.send] attempt=$ATTEMPT partial_reply=${REPLY:0:40}"
    fi
done

echo "[sexnet.phaseE.host_udp.fail] attempts=$MAX_ATTEMPTS target=$TARGET_IP:$TARGET_PORT"
exit 0
