#!/bin/bash
set -uo pipefail

TAP_IF="${QEMU_TAP_IFNAME:-tap0}"
GUEST_IP="10.0.2.15"
LOG="/tmp/sexnet_arp_host_observe.log"

: > "$LOG"

echo "[arp.host.probe.begin] tap=$TAP_IF guest=$GUEST_IP" | tee -a "$LOG"

sudo ip neigh flush dev "$TAP_IF" 2>/dev/null || true

sudo arping -I "$TAP_IF" -c 10 -w 30 "$GUEST_IP" 2>&1 | tee -a "$LOG" || true

ip neigh show "$GUEST_IP" dev "$TAP_IF" 2>/dev/null | tee -a "$LOG" || true

reply_seen=0
if grep -qi "Unicast reply\|bytes from" "$LOG"; then
  reply_seen=1
elif ip neigh show "$GUEST_IP" dev "$TAP_IF" 2>/dev/null | grep -q "lladdr"; then
  reply_seen=1
fi

if [ "$reply_seen" -eq 1 ]; then
  echo "[arp.host.observe.proof.done] reply_seen=1 ok=1" | tee -a "$LOG"
  echo "PASS" | tee -a "$LOG"
  exit 0
else
  echo "[arp.host.observe.proof.done] reply_seen=0 ok=0" | tee -a "$LOG"
  echo "FAIL -- check TAP setup or sexnet boot timing" | tee -a "$LOG"
  exit 1
fi
