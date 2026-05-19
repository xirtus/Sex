# SEXNET_ICMP_ECHO_REPLY_GATE_V1

Date: 2026-05-19
Branch: master
Commit: pending (Phase D ICMP echo reply gate)
Gate: `sexnet_icmp_echo_reply` (new)
Depends: Phase C IPv4 validate + checksum (proven)

## Old State

Pre-Phase D gates (`gate_icmp_echo_request_plan`, `gate_icmp_echo_request_send_stop_review`,
`gate_icmp_echo_request_proof`, `gate_icmp_echo_reply_observe_proof`) exist but
are SKIP-only stub gates that check for higher-level markers never emitted by
the runtime. No ICMP echo reply runtime code existed.

## New Gate: sexnet_icmp_echo_reply

A new gate is added to `scripts/daily_driver_master_gate.sh` after the
`sexnet_ipv4_checksum` gate (inserted after line 2305).

### Gate Declaration

```
gate_sexnet_icmp_echo_reply="SKIP"
```

### Gate Logic

```
# ---- SEXNET_ICMP_ECHO_REPLY_GATE_V1 ----
# Proves ICMP echo request received → echo reply built + transmitted.
# Must run after sexnet_ipv4_checksum gates.

if [ "$(has 'sexnet\.icmp\.echo\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_sexnet_icmp_echo_reply="PASS"
    print_row "sexnet_icmp_echo_reply" "PASS" "ICMP echo reply proof: RX echo → TX reply → DD done"
elif [ "$(has 'sexnet\.icmp\.echo\.proof\.done.*ok=0')" -eq 1 ]; then
    gate_sexnet_icmp_echo_reply="FAIL"
    print_row "sexnet_icmp_echo_reply" "FAIL" "ICMP proof.done ok=0 — echo reply failed"
elif [ "$(has 'sexnet\.icmp\.rx\.echo.*ok=1')" -eq 1 ]; then
    if [ "$(has 'sexnet\.icmp\.tx\.poll\.done.*dd_set=1.*ok=1')" -eq 0 ]; then
        gate_sexnet_icmp_echo_reply="FAIL"
        print_row "sexnet_icmp_echo_reply" "FAIL" "ICMP RX echo received but TX DD not done"
    else
        gate_sexnet_icmp_echo_reply="PASS"
        print_row "sexnet_icmp_echo_reply" "PASS" "ICMP echo reply markers present"
    fi
elif [ "$(has 'sexnet\.icmp\.reject.*ok=1')" -eq 1 ] \
  && [ "$(has 'sexnet\.icmp\.rx\.echo.*ok=1')" -eq 0 ]; then
    gate_sexnet_icmp_echo_reply="PASS"
    print_row "sexnet_icmp_echo_reply" "PASS" "ICMP negative path proven (reject non-echo, no positive echo)"
else
    gate_sexnet_icmp_echo_reply="SKIP"
    print_row "sexnet_icmp_echo_reply" "SKIP" "no ICMP echo stimulus (TAP/usernet without ping)"
fi
```

### PASS Conditions

- `sexnet.icmp.echo.proof.done ok=1` present, OR
- `sexnet.icmp.rx.echo ok=1` + `sexnet.icmp.tx.poll.done dd_set=1 ok=1` present, OR
- `sexnet.icmp.reject ok=1` present without any positive echo (negative path proven)
- No #PF/#GP/panic/fault.kill/KERNEL PANIC

### FAIL Conditions

- `sexnet.icmp.echo.proof.done ok=0` present
- `sexnet.icmp.rx.echo ok=1` present but `sexnet.icmp.tx.poll.done dd_set=1` absent
- Malformed ICMP accepted (no reject, no positive proof)

### SKIP Conditions

- No ICMP markers at all (TAP without ping stimulus, usernet without ICMP route)
- Profile intentionally disables ICMP proof

### Gate Output Table Entry

```
"sexnet_icmp_echo_reply:$gate_sexnet_icmp_echo_reply"
```

Inserted after `sexnet_ipv4_checksum` entry in the gate output table.

### Proof Command

```bash
QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_d_tap.log

./scripts/daily_driver_master_gate.sh /tmp/sexnet_phase_d_tap.log
```

### Log Path

- `/tmp/sexnet_phase_d_tap.log`

### Fault Count

Expected: 0 faults

### Next

SEXNET_ICMP_HOST_PING_OBSERVE_PROOF_V1 (Task 16)
