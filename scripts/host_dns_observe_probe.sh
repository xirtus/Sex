#!/usr/bin/env bash
# host_dns_observe_probe.sh — bounded host-side DNS observe probe
# Probes whether the host can resolve example.com A record via system DNS.
# Used by Phase F as companion evidence for DNS client proof.
# This probe does NOT interact with the QEMU guest; it only checks host DNS.
# Output log path required as first argument.
set -euo pipefail

LOG="${1:-/tmp/sexnet_phase_f_host_dns.log}"

echo "[host.dns.probe.begin] host=example.com qtype=A" | tee "$LOG"

# Try dig first (most common)
if command -v dig &>/dev/null; then
    RESULT=$(dig +short +time=3 +tries=1 example.com A 2>&1) || true
    if [ -n "$RESULT" ]; then
        echo "[host.dns.probe.resolve] host=example.com resolved=1 method=dig ips=$RESULT" | tee -a "$LOG"
        echo "[host.dns.probe.done] resolved=1 fake=0 ok=1 reason=host_dns_resolved_via_dig" | tee -a "$LOG"
        exit 0
    fi
fi

# Try nslookup
if command -v nslookup &>/dev/null; then
    RESULT=$(nslookup example.com 2>&1) || true
    if echo "$RESULT" | grep -q "Address:"; then
        IPS=$(echo "$RESULT" | grep "Address:" | grep -v "#" | awk '{print $2}' | tr '\n' ' ')
        echo "[host.dns.probe.resolve] host=example.com resolved=1 method=nslookup ips=$IPS" | tee -a "$LOG"
        echo "[host.dns.probe.done] resolved=1 fake=0 ok=1 reason=host_dns_resolved_via_nslookup" | tee -a "$LOG"
        exit 0
    fi
fi

# Try host command
if command -v host &>/dev/null; then
    RESULT=$(host example.com 2>&1) || true
    if echo "$RESULT" | grep -q "has address"; then
        IPS=$(echo "$RESULT" | grep "has address" | awk '{print $4}' | tr '\n' ' ')
        echo "[host.dns.probe.resolve] host=example.com resolved=1 method=host ips=$IPS" | tee -a "$LOG"
        echo "[host.dns.probe.done] resolved=1 fake=0 ok=1 reason=host_dns_resolved_via_host" | tee -a "$LOG"
        exit 0
    fi
fi

# Try getent (glibc hosts/DNS)
if command -v getent &>/dev/null; then
    RESULT=$(getent ahosts example.com 2>&1) || true
    if [ -n "$RESULT" ]; then
        IPS=$(echo "$RESULT" | awk '{print $1}' | tr '\n' ' ')
        echo "[host.dns.probe.resolve] host=example.com resolved=1 method=getent ips=$IPS" | tee -a "$LOG"
        echo "[host.dns.probe.done] resolved=1 fake=0 ok=1 reason=host_dns_resolved_via_getent" | tee -a "$LOG"
        exit 0
    fi
fi

# All methods failed
echo "[host.dns.probe.done] resolved=0 fake=0 ok=0 reason=no_host_dns_tool_available_or_no_resolution" | tee -a "$LOG"
exit 0
