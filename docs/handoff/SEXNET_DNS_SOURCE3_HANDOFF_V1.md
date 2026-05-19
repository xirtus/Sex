# SEXNET_DNS_SOURCE3_HANDOFF_V1

Date: 2026-05-20
Mission: SEXNET_SOURCE3_DNS_P6_GATES_HANDOFF_V1

## Status

DNS migration proof ownership is now wired into daily-driver gates and handoff docs.
No feature/runtime behavior code was changed in this mission.

Result classification:
- Source3 DNS PASS when query build + tx_dd=1 + rx parse/answer + cache insert + browser resolve ok + legacy source2 not_used + no faults.
- Source3 DNS SKIP when environment returns no DNS response and markers explicitly show timeout/cache-miss.
- Source3 DNS FAIL on malformed accepted, fabricated browser IP, source2 DNS proof use, tx_dd=0, or any fault marker.

## Files Changed

- `scripts/daily_driver_master_gate.sh`
- `docs/handoff/SEXNET_DNS_SOURCE3_HANDOFF_V1.md` (new)
- `docs/handoff/NETWORK_SPRINT_EXECUTION_V1.md`
- `docs/handoff/NETWORK_STACK_STATUS_ROLLUP_V1.md`
- `docs/handoff/SEXNET_SOURCE3_NETWORK_100_RELEASE_NOTE_V1.md`

## Ownership

- Runtime source of DNS markers: `servers/sexnet/src/main.rs`
- Browser resolve markers: `apps/kaleidoscope/src/main.rs`
- Gate/policy enforcement: `scripts/daily_driver_master_gate.sh`
- Handoff/status documentation: `docs/handoff/*`

## Proof Markers

Required PASS-lane markers:
- `[sexnet.dns.source3.query.build] ... ok=1`
- `[sexnet.dns.source3.udp.tx] ... tx_dd=1 ok=1`
- `[sexnet.dns.source3.rx.parse] ... ok=1`
- `[sexnet.dns.source3.answer.a] ... ok=1`
- `[sexnet.dns.source3.cache.insert] ... ok=1`
- `[browser.dns.resolve.request] ... ok=1`
- `[browser.dns.resolve.ok] ... ok=1`
- `[legacy.source2.dns.not_used] ... ok=1`

Allowed SKIP-lane markers:
- `[sexnet.dns.source3.rx.timeout] ... reason=no_response_env_blocked`
- `[browser.dns.resolve.miss] ... reason=cache_miss`

FAIL triggers:
- `[sexnet.dns.malformed.accepted]` present
- `browser.dns.resolve.ok` without source3 answer/cache evidence
- source2 DNS proof markers used in source3 lane (`sexnet.dns.query.build/query.tx/response.parse/cache.*`)
- `[sexnet.dns.source3.udp.tx] ... tx_dd=0` or `ok=0`
- any `#PF/#GP/panic/fault.kill` via `faults_zero`

## Gate Policy (Implemented)

New gates added in `scripts/daily_driver_master_gate.sh`:
- `sexnet_dns_source3_query_build`
- `sexnet_dns_source3_udp_tx`
- `sexnet_dns_source3_rx_parse_or_timeout`
- `sexnet_dns_source3_cache_insert_or_timeout`
- `sexnet_dns_source3_browser_resolve`
- `sexnet_dns_source3_legacy_source2_not_used`
- `sexnet_dns_source3_proof_v1` (rollup)

Policy summary:
- PASS requires complete source3 marker chain + no faults.
- SKIP is allowed only for explicit no-response/cache-miss env-blocked lane.
- FAIL on policy violations listed above.

## STOP FIRST Boundaries

STOP FIRST if:
- Gate script structure becomes ambiguous for this lane.
- Required docs/handoff paths are missing.
- Enforcing policy requires feature/runtime behavior changes.
- Any kernel/ABI edits appear required.
- SKIP semantics cannot be expressed without broad script rewrite.

## Deferred Items

- HAL source2 DNS path remains frozen legacy and undeleted.
- Realtek/real hardware DNS remains deferred.
- TLS and broader resolver stack remain out of this mission.

## Proof Target Constraints

- QEMU e1000 + SLiRP DNS target only: `10.0.2.3:53`
- No browser raw NIC path
- No kernel edits
- No sex-pdx ABI edits

## Exact Proof Commands

```bash
./scripts/entrypoint_build.sh

# Run proof profile/log generation as used by your lane, then evaluate gates:
./scripts/daily_driver_master_gate.sh /tmp/sexnet_phase_o_final_network.log

# Marker audit for source3 DNS migration wiring:
rg -n "sexnet_dns_source3|dns.source3|browser.dns.resolve|legacy.source2.dns.not_used|SEXNET_DNS_SOURCE3_HANDOFF" \
  scripts docs claude-references servers/sexnet/src/main.rs apps/kaleidoscope/src/main.rs
```
