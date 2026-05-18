# SEXNET_ARP_REQUEST_REPLY_GATE_V1

## A. Result
One-shot `sexnet` ARP request/reply gate is proven in the TAP/e1000e lane with poll-driven RX/TX descriptor flow and zero faults.

Proof truth for this gate:
- This is one-shot ARP request/reply only.
- This is NOT an ARP cache.
- This is NOT IP.
- This is NOT TCP/HTTP/DNS.
- This is NOT routing.
- This is poll-driven, not IRQ-driven.
- `NET_DIAG`/browser HTTP path still comes from HAL boot diagnostic atomics.

## B. Proof command / host preconditions
Host preconditions (must be TAP backend, not slirp):
- `QEMU_NET_BACKEND=tap`
- `QEMU_NET_MODEL=e1000e`
- `QEMU_TAP_IFNAME=tap0`
- `ENABLE_QEMU_USERNET_E1000=1`

Host ARP stimulus loop (run before/during QEMU):
```bash
while true; do
  sudo arping -I tap0 -c 1 10.0.2.15 2>/dev/null || true
  sleep 0.05
done
```

Proof run command:
```bash
QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
./scripts/run_daily_driver_proof.sh /tmp/sexnet_arp_request_reply_gate_v1.log
```

Scan command:
```bash
grep -E "sexnet_arp_|sexnet.arp|fault.kill|#PF|#GP|panic|KERNEL PANIC|FINAL:" \
/tmp/sexnet_arp_request_reply_gate_v1.log | tail -420
```

## C. Marker evidence
Observed proven markers:
- `[sexnet.l2.proof.done] rx_frames=1 tx_dd=1 ok=1`
- `[sexnet.arp.rx.poll.begin] max_iters=10000000`
- `[sexnet.arp.rx.frame] idx=2 ethertype=0x0806 ok=1`
- `[sexnet.arp.rx.validate] htype=1 ptype=0x0800 hlen=6 plen=4 oper=1 tpa_match=1 ok=1`
- `[sexnet.arp.tx.reply.build] spa=10.0.2.15 ok=1`
- `[sexnet.arp.tx.desc] slot=1 len=60 ok=1`
- `[sexnet.arp.tx.post] tdt=2 ok=1`
- `[sexnet.arp.tx.poll.done] dd_set=1 ok=1`
- `[sexnet.arp.proof.done] rx_arp=1 tx_dd=1 ok=1`
- `FINAL: PASS (256 gates proved, 18 skipped, 0 faults)`

## D. What was proven
- `sexnet` L2 loop passed with bounded request/reply marker chain.
- Guest received a real ARP request for `10.0.2.15`.
- ARP request field validation passed (`htype=1`, `ptype=0x0800`, `hlen=6`, `plen=4`, `oper=1`, `tpa_match=1`).
- Guest built and posted ARP reply on TX descriptor slot `1` (`len=60`, `tdt=2`).
- L2 reuse moved after ARP reply and now proves TX descriptor slot `2` (`len=60`, `tdt=3`, `desc_idx=2`).
- NIC consumed posted TX descriptor (`dd_set=1`).
- No forced creep into IP/TCP/HTTP/cache logic.
- No kernel faults in this proof lane.

## E. What was not proven
- Host `arping` stdout capture of received reply packet is not proven in this gate.
- No ARP cache behavior is proven.
- No IPv4/ICMP, TCP/HTTP, DNS, routing, or browser-owned network behavior is proven.
- No IRQ-driven path is proven.
- Browser/`NET_DIAG` source behavior was not replaced by this mission.

## F. Architecture boundary
This gate is docs/gate-only around existing runtime markers. It does not authorize kernel, PCI HAL, `sex-pdx`, build system, browser/display, or route-semantic expansion.

## G. STOP FIRST rules
STOP FIRST if any of the following becomes required:
- IP/TCP/HTTP/DNS markers to satisfy this ARP gate.
- Browser/HAL code changes to satisfy this ARP gate.
- Hard-fail behavior on non-TAP/no-ARP boots (must be SKIP in that lane).
- Marker rename/source edits in `servers/sexnet/src/main.rs`.
- Any code change outside allowed gate/docs files.

## H. Next missions
1. `SEXNET_ARP_REPLY_HOST_OBSERVE_PROOF_V1`
- Optional: prove host `arping` receives reply, if stdout capture is feasible.

2. `SEXNET_ARP_CACHE_STOP_REVIEW_V1`
- Review minimal cache only after this one-shot gate is frozen.

3. `SEXNET_IPV4_ECHO_STOP_REVIEW_V1`
- Review IPv4/ICMP only after ARP cache plan is accepted.

4. `SEXNET_NETDIAG_SOURCE3_PLAN_V1`
- Plan `sexnet`-owned diagnostic source path.
