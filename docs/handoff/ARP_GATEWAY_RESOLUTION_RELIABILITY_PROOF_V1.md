# ARP_GATEWAY_RESOLUTION_RELIABILITY_PROOF_V1

## Scope
- e1000e proof lane only (`QEMU_NET_MODEL=e1000e`, `ENABLE_QEMU_USERNET_E1000=1`).
- No fake gateway MAC.
- TCP SYN remains hard-gated by `gateway_known` and nonzero `gw_mac`.
- No RDH write added in the gateway resolver path.

## Retry Table
| attempt | target_ip | tx_dd | poll_rounds_budget | poll_rounds_used | reply_seen | reason |
|---|---|---:|---:|---:|---:|---|
| 1 | 10.0.2.2 | 1 | 64 | 1 | 1 | valid_arp_reply_observed |

## RX Reply Table
| attempt | SPA | TPA | MAC | ethertype | htype | ptype | hlen | plen | oper | SHA nonzero | fake | valid |
|---|---|---|---|---|---|---|---|---|---|---|---:|---:|
| 1 | 10.0.2.2 | 10.0.2.15 | 52:55:0A:00:02:02 | 0x0806 | 1 | 0x0800 | 6 | 4 | 2 | yes | 0 | 1 |

## Gateway Truth
- `[arp.gateway.resolved] gateway_known=1 gw_mac=52:55:0A:00:02:02 attempts=1 fake=0 ok=1 reason=resolved_from_real_arp_reply`
- `[arp.gateway.resolution.reliability.done] ok=1 gateway_known=1 attempts=1 fake=0`

## TCP Precondition Truth
- SYN precondition still enforced in code path: no SYN send when `gateway_known=0`.
- In this run, gateway resolved first, then SYN attempts occurred (`syn_sent=1`) with nonzero gateway MAC.
- No HTTP send occurred in this run:
  - `[http.get.send.stop.review] stop=1 reason=tcp_connect_not_completed`
  - `[http.get.send.proof] sent=0 tx_dd=0 payload_len=0 ok=0 reason=no_final_ack_no_http_send`

## Proof Result
- Build: `./scripts/entrypoint_build.sh` passed.
- Runtime proof: `QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 ./scripts/run_daily_driver_proof.sh /tmp/sexos_arp_gateway_resolution_reliability_proof_v1.log`
- Gate verdict from proof runner: `FINAL: PASS` (`PASS gates: 236`, `FAIL gates: 0`, `SKIP gates: 12`).
- Network reliability gate: `arp_gateway_resolution_reliability PASS`.

## Fault Count
- 0 faults (`faults_zero PASS`; `network.fault.containment.proof crash_events=0`).

## Next
- If `gateway_known=1`: `TCP_SYN_SEND_RETRY_PROOF_V1`
- If `gateway_known=0`: `QEMU_SLIRP_ARP_STABILITY_PROBE_V1`
