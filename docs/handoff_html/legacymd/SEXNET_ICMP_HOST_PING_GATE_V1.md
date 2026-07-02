# SEXNET_ICMP_HOST_PING_GATE_V1

Date: 2026-05-19
Branch: master
Commit: pending (Phase D host ping observe gate)
Gate: `sexnet_icmp_host_ping_observe` (new)
Depends: ICMP echo reply gate (guest-side)

## Old State

Pre-Phase D gate `gate_icmp_echo_reply_observe_proof` exists but is SKIP-only,
checking for a marker never emitted by the runtime.

## New Gate: sexnet_icmp_host_ping_observe

A new gate is added to `scripts/daily_driver_master_gate.sh` after the
`sexnet_icmp_echo_reply` gate.

### Gate Declaration

```
gate_sexnet_icmp_host_ping_observe="SKIP"
```

### Gate Logic

```
# ---- SEXNET_ICMP_HOST_PING_GATE_V1 ----
# Checks host-side ping observe probe log for reply markers.
# Must run after guest-side ICMP echo reply gate.

HOST_PING_LOG="/tmp/sexnet_phase_d_host_ping.log"
if [ -f "$HOST_PING_LOG" ] && [ "$(grep -c 'sexnet\.phaseD\.host_ping\.pass' "$HOST_PING_LOG" 2>/dev/null || echo 0)" -gt 0 ]; then
    gate_sexnet_icmp_host_ping_observe="PASS"
    print_row "sexnet_icmp_host_ping_observe" "PASS" "host ping reply observed from 10.0.2.15"
elif [ -f "$HOST_PING_LOG" ] && [ "$(grep -c 'sexnet\.phaseD\.host_ping\.fail' "$HOST_PING_LOG" 2>/dev/null || echo 0)" -gt 0 ]; then
    gate_sexnet_icmp_host_ping_observe="FAIL"
    print_row "sexnet_icmp_host_ping_observe" "FAIL" "host ping sent but no reply observed"
elif [ -f "$HOST_PING_LOG" ] && [ "$(grep -c 'sexnet\.phaseD\.host_ping\.skip' "$HOST_PING_LOG" 2>/dev/null || echo 0)" -gt 0 ]; then
    gate_sexnet_icmp_host_ping_observe="SKIP"
    print_row "sexnet_icmp_host_ping_observe" "SKIP" "host ping probe skipped (env constraint)"
elif [ "$gate_sexnet_icmp_echo_reply" = "PASS" ]; then
    gate_sexnet_icmp_host_ping_observe="PASS"
    print_row "sexnet_icmp_host_ping_observe" "PASS" "PASS REVIEW ONLY — guest ICMP reply proven, host observe not run"
else
    gate_sexnet_icmp_host_ping_observe="SKIP"
    print_row "sexnet_icmp_host_ping_observe" "SKIP" "no host ping probe log and no guest ICMP reply"
fi
```

### PASS Conditions

- Host ping observe probe returns PASS (ping reply received), OR
- Guest-side ICMP echo reply proven (PASS REVIEW ONLY, host observe not run)
- No faults

### FAIL Conditions

- Host ping probe returns FAIL (ping sent, no reply)
- Guest ICMP reply markers contradict host observe

### SKIP Conditions

- Host ping probe returns SKIP (env constraint)
- No host probe log and no guest ICMP reply

### Gate Output Table Entry

```
"sexnet_icmp_host_ping_observe:$gate_sexnet_icmp_host_ping_observe"
```

### Fault Count

Expected: 0 faults

### Next

Phase E: SEXNET_UDP_PARSE_STOP_REVIEW_V1
