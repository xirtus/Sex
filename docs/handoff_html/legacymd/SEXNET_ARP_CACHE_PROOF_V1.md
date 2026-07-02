# SEXNET_ARP_CACHE_PROOF_V1

Date: 2026-05-19
Commit baseline: `2957a37` (`sexnet: prove bounded ARP cache replies`)
Mission: `SEXNET_ARP_CACHE_GATE_AND_HANDOFF_V1`

## A. Result
- Gate added: `sexnet_arp_cache_proof` in `scripts/daily_driver_master_gate.sh`.
- Handoff created for bounded ARP cache proof markers and stop-rules.
- Scope stayed gate/docs-only.

## B. Proof command / host preconditions
Host preconditions:
- TAP backend available and configured (`tap0`).
- ARP stimulus running on host.

Start ARP flood on host:
```bash
while true; do
  sudo arping -I tap0 -c 1 -w 1 10.0.2.15 2>/dev/null || true
  sleep 0.05
done
```

Run proof:
```bash
QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
./scripts/run_daily_driver_proof.sh /tmp/sexnet_arp_cache_gate_and_handoff_v1.log
```

Scan evidence:
```bash
grep -E "sexnet_arp_cache|sexnet.arp.cache|fault.kill|#PF|#GP|panic|KERNEL PANIC|FINAL:" \
/tmp/sexnet_arp_cache_gate_and_handoff_v1.log | tail -420
```

## C. Marker evidence
Expected bounded markers:
- `[sexnet.arp.cache.poll.begin] max_iters=100000000 target_replies=2`
- `[sexnet.arp.cache.learn] n=1 ... ok=1`
- `[sexnet.arp.cache.reply] n=1 slot=3 tdt=4 ok=1`
- `[sexnet.arp.cache.reply.dd] n=1 dd_set=1 ok=1`
- `[sexnet.arp.cache.learn] n=2 ... ok=1`
- `[sexnet.arp.cache.reply] n=2 slot=4 tdt=5 ok=1`
- `[sexnet.arp.cache.reply.dd] n=2 dd_set=1 ok=1`
- `[sexnet.arp.cache.poll.done] ... replies=2 ok=1`
- `[sexnet.arp.cache.proof.done] replies=2 ok=1`

Current proven run (baseline):
- Final result reported: `FINAL: PASS (262 gates proved, 17 skipped, 0 faults)`.

## D. What was proven
- Bounded poll-driven ARP cache behavior reached two reply events.
- Cache learn and reply markers reached `ok=1`.
- TX descriptor completion (`dd_set=1`) observed for both bounded reply events (`n=1`, `n=2`).
- Expected reply slot/TDT progression was bounded and explicit:
  - `n=1 slot=3 tdt=4`
  - `n=2 slot=4 tdt=5`

## E. What was not proven
- Not a long-running ARP daemon.
- Only bounded 1-entry cache behavior was proven.
- No IRQ-driven ARP flow proof (poll-driven lane only).
- No IP/ICMP/TCP/HTTP/DNS proof in this mission.
- Browser and `NET_DIAG` flows are not replaced by this gate.

## F. Architecture boundary
- This mission only adds gate logic and handoff docs.
- No changes to `servers/sexnet/src/main.rs`, kernel, HAL, `sex-pdx`, browser/display, or build/Cargo/limine surfaces.

## G. STOP FIRST rules
Stop immediately if any of the following is required:
- Source code changes to satisfy marker contracts.
- Gate behavior that hard-fails non-TAP / no-traffic boots (must SKIP when markers are absent).
- Any scope expansion into IP/ICMP/TCP/HTTP/DNS.
- Marker rename or schema drift that would require producer-side edits.

## H. Next missions
- `SEXNET_IPV4_PARSE_STOP_REVIEW_V1`
