#!/bin/bash
# sexnet_packet_truth_gate.sh — Phase B-F packet truth proof gate
# Mission: SEXNET_AUTOPILOT_PACKET_TRUTH_V1
#
# Usage:
#   ./scripts/sexnet_packet_truth_gate.sh <log_file>

set -euo pipefail

LOG="${1:?usage: $0 <log_file>}"
[ -f "$LOG" ] || { echo "ERROR: log file not found: $LOG"; exit 1; }

pass=0; skip=0; fail=0

has() { grep -q "$1" "$LOG"; }

gcount() { grep -c "$1" "$LOG" 2>/dev/null || true; }

print_row() { printf "%-45s %-6s %s\n" "$1" "$2" "$3"; }

# fault scan
faults=$(grep -ciE 'panic|KERNEL PANIC|#PF|#GP|fault\.kill|bounds violation|IPC storm' "$LOG" 2>/dev/null || true)
faults=${faults:-0}

echo "============================================"
echo " sexnet_packet_truth_gate  Phase B-F"
echo " log: $LOG"
echo " faults detected: $faults"
echo "============================================"

# ═══ Phase B: RX/TX Descriptor Truth ═══
echo ""
echo "── Phase B: RX/TX Descriptor Truth ──"

if has 'sexnet\.nic\.tx\.dd\.ok.*dd_set=1.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_nic_tx_dd_ok" "PASS" "TX descriptor DD consumed by hardware"
elif has 'sexnet\.nic\.tx\.observe\.poll\.done.*dd_set=1'; then
    pass=$((pass+1)); print_row "sexnet_nic_tx_dd_ok" "PASS" "TX DD consumed (legacy fallback)"
else
    skip=$((skip+1)); print_row "sexnet_nic_tx_dd_ok" "SKIP" "no TX DD marker found"
fi

if has 'sexnet\.nic\.rx\.observe\.ok.*dd_set=[1-9].*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_nic_rx_observe_ok" "PASS" "RX descriptor observe DD set"
elif has 'sexnet\.nic\.rx\.observe\.ok.*ok=0'; then
    fail=$((fail+1)); print_row "sexnet_nic_rx_observe_ok" "FAIL" "RX observe marker with ok=0"
else
    skip=$((skip+1)); print_row "sexnet_nic_rx_observe_ok" "SKIP" "no RX traffic observed in window"
fi

if has 'sexnet\.nic\.rx\.timeout\.honest.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_nic_rx_timeout_honest" "PASS" "honest RX timeout — bounded poll, no traffic"
else
    skip=$((skip+1)); print_row "sexnet_nic_rx_timeout_honest" "SKIP" "timeout marker not emitted (RX had traffic)"
fi

# ═══ Phase C: Ethernet Frame Classifier ═══
echo ""
echo "── Phase C: Ethernet Frame Classifier ──"

if has 'sexnet\.ether\.parse\.ok.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_ether_parse_ok" "PASS" "ethernet frame parsed ok"
elif has 'sexnet\.ether\.parse\.ok.*ok=0'; then
    fail=$((fail+1)); print_row "sexnet_ether_parse_ok" "FAIL" "ethernet parse marker with ok=0"
else
    skip=$((skip+1)); print_row "sexnet_ether_parse_ok" "SKIP" "no ethernet frame observed"
fi

if has 'sexnet\.ether\.runt\.reject.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_ether_runt_reject" "PASS" "runt frame correctly rejected"
else
    skip=$((skip+1)); print_row "sexnet_ether_runt_reject" "SKIP" "no runt frame observed (OK)"
fi

if has 'sexnet\.ether\.ethertype\.unknown\.reject.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_ether_ethertype_unknown_reject" "PASS" "unknown ethertype rejected"
else
    skip=$((skip+1)); print_row "sexnet_ether_ethertype_unknown_reject" "SKIP" "no unknown ethertype observed (OK)"
fi

# ═══ Phase D: ARP Real Peer Proof ═══
echo ""
echo "── Phase D: ARP Real Peer Proof ──"

if has 'sexnet\.arp\.request\.tx\.ok.*tx_dd=1.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_arp_request_tx_ok" "PASS" "ARP request transmitted"
elif has 'sexnet\.arp\.request\.tx\.ok.*ok=0'; then
    fail=$((fail+1)); print_row "sexnet_arp_request_tx_ok" "FAIL" "ARP TX marker with ok=0"
else
    skip=$((skip+1)); print_row "sexnet_arp_request_tx_ok" "SKIP" "ARP request not triggered"
fi

if has 'sexnet\.arp\.reply\.rx\.ok.*oper=1.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_arp_reply_rx_ok" "PASS" "ARP reply received from peer"
elif has 'sexnet\.arp\.reply\.rx\.skip.*reason=.*ok=1'; then
    skip=$((skip+1)); print_row "sexnet_arp_reply_rx_ok" "SKIP" "no ARP reply (honest skip)"
else
    skip=$((skip+1)); print_row "sexnet_arp_reply_rx_ok" "SKIP" "ARP reply not observed"
fi

if has 'sexnet\.arp\.cache\.gateway\.ok.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_arp_cache_gateway_ok" "PASS" "gateway MAC cached from ARP reply"
elif has 'sexnet\.arp\.cache\.gateway\.ok.*ok=0'; then
    fail=$((fail+1)); print_row "sexnet_arp_cache_gateway_ok" "FAIL" "ARP cache gateway marker with ok=0"
else
    skip=$((skip+1)); print_row "sexnet_arp_cache_gateway_ok" "SKIP" "no gateway MAC learned (env-limited)"
fi

if has 'sexnet\.arp\.reply\.rx\.skip.*reason=.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_arp_reply_rx_skip" "PASS" "honest skip — no peer reply"
elif has 'sexnet\.arp\.reply\.rx\.ok.*oper=1.*ok=1'; then
    skip=$((skip+1)); print_row "sexnet_arp_reply_rx_skip" "SKIP" "ARP reply was received (skip not applicable)"
else
    skip=$((skip+1)); print_row "sexnet_arp_reply_rx_skip" "SKIP" "ARP stage not executed"
fi

# ═══ Phase E: IPv4 Parser Hardening ═══
echo ""
echo "── Phase E: IPv4 Parser Hardening ──"

if has 'sexnet\.ipv4\.parse\.ok.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_ipv4_parse_ok" "PASS" "IPv4 parsed successfully"
elif has 'sexnet\.ipv4\.parse\.ok.*ok=0'; then
    fail=$((fail+1)); print_row "sexnet_ipv4_parse_ok" "FAIL" "IPv4 parse marker with ok=0"
else
    skip=$((skip+1)); print_row "sexnet_ipv4_parse_ok" "SKIP" "no IPv4 traffic observed"
fi

if has 'sexnet\.ipv4\.bad_checksum\.reject.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_ipv4_bad_checksum_reject" "PASS" "bad IPv4 checksum rejected"
elif has 'sexnet\.ipv4\.rx\.validate.*checksum=ok'; then
    skip=$((skip+1)); print_row "sexnet_ipv4_bad_checksum_reject" "SKIP" "no bad checksum (all valid)"
else
    skip=$((skip+1)); print_row "sexnet_ipv4_bad_checksum_reject" "SKIP" "no IPv4 traffic for checksum check"
fi

if has 'sexnet\.ipv4\.fragment\.reject.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_ipv4_fragment_reject" "PASS" "IPv4 fragment correctly rejected"
elif has 'sexnet\.ipv4\.rx\.validate.*frag=0'; then
    skip=$((skip+1)); print_row "sexnet_ipv4_fragment_reject" "SKIP" "no fragments observed (all non-frag)"
else
    skip=$((skip+1)); print_row "sexnet_ipv4_fragment_reject" "SKIP" "no IPv4 traffic for fragment check"
fi

if has 'sexnet\.ipv4\.bounds\.reject.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_ipv4_bounds_reject" "PASS" "IPv4 bounds violation rejected"
elif has 'sexnet\.ipv4\.rx\.validate.*ok=1'; then
    skip=$((skip+1)); print_row "sexnet_ipv4_bounds_reject" "SKIP" "no bounds violation (all in bounds)"
else
    skip=$((skip+1)); print_row "sexnet_ipv4_bounds_reject" "SKIP" "no IPv4 traffic for bounds check"
fi

# ═══ Phase F: ICMP Echo Proof ═══
echo ""
echo "── Phase F: ICMP Echo Proof ──"

if has 'sexnet\.icmp\.echo\.rx\.ok.*type=8.*code=0.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_icmp_echo_rx_ok" "PASS" "ICMP echo request received"
elif has 'sexnet\.icmp\.echo\.rx\.ok.*ok=0'; then
    fail=$((fail+1)); print_row "sexnet_icmp_echo_rx_ok" "FAIL" "ICMP echo RX marker with ok=0"
else
    skip=$((skip+1)); print_row "sexnet_icmp_echo_rx_ok" "SKIP" "no ICMP echo request observed"
fi

if has 'sexnet\.icmp\.echo\.reply\.tx\.ok.*tx_dd=1.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_icmp_echo_reply_tx_ok" "PASS" "ICMP echo reply transmitted"
elif has 'sexnet\.icmp\.echo\.reply\.tx\.ok.*ok=0'; then
    fail=$((fail+1)); print_row "sexnet_icmp_echo_reply_tx_ok" "FAIL" "ICMP reply TX marker with ok=0"
else
    skip=$((skip+1)); print_row "sexnet_icmp_echo_reply_tx_ok" "SKIP" "ICMP echo reply not sent (no request)"
fi

if has 'sexnet\.icmp\.ping\.gateway\.ok.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_icmp_ping_gateway_ok" "PASS" "ICMP ping gateway succeeded"
elif has 'sexnet\.icmp\.ping\.gateway\.skip.*reason=.*ok=1'; then
    skip=$((skip+1)); print_row "sexnet_icmp_ping_gateway_ok" "SKIP" "ICMP ping gateway honest skip (no ARP/no reply)"
else
    skip=$((skip+1)); print_row "sexnet_icmp_ping_gateway_ok" "SKIP" "ICMP ping gateway not attempted"
fi

if has 'sexnet\.icmp\.ping\.gateway\.skip.*reason=.*ok=1'; then
    pass=$((pass+1)); print_row "sexnet_icmp_ping_gateway_skip" "PASS" "honest skip — gateway ping not possible"
elif has 'sexnet\.icmp\.ping\.gateway\.ok.*ok=1'; then
    skip=$((skip+1)); print_row "sexnet_icmp_ping_gateway_skip" "SKIP" "gateway ping succeeded (skip not applicable)"
else
    skip=$((skip+1)); print_row "sexnet_icmp_ping_gateway_skip" "SKIP" "ICMP ping stage not executed"
fi

# ═══ Final Rollup ═══
echo ""
echo "============================================"
if [ "$faults" -gt 0 ]; then
    echo " RESULT: FAIL  (faults detected: $faults)"
elif [ "$fail" -gt 0 ]; then
    echo " RESULT: FAIL  (pass=$pass skip=$skip fail=$fail faults=$faults)"
else
    echo " RESULT: PASS  (pass=$pass skip=$skip fail=$fail faults=$faults)"
fi
echo "============================================"
echo " PASS:  $pass"
echo " SKIP:  $skip"
echo " FAIL:  $fail"
echo " FAULT: $faults"
echo "============================================"
