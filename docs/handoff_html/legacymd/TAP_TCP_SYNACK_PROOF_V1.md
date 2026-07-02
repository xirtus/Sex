# TAP_TCP_SYNACK_PROOF_V1

## Mission Result
**STOP: Missing TCP Response under TAP due to ARP Regression (Subnet Mismatch)**

## Proof Execution
```bash
QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
./scripts/run_daily_driver_proof.sh /tmp/sexos_tap_network_boot.log
```

## Log Analysis & Markers
- **Log Path:** `/tmp/sexos_tap_network_boot.log`
- `[net.tap.backend.active]` / TAP active evidence: QEMU started with `backend=tap` and `tap_if=tap0`.
- `[tcp.syn.tx.done]` / TCP SYN TX: Missing. The log shows `[tcp.syn.tx.post] ... tx_dd=0 syn_sent=0 ... reason=gateway_unknown_no_syn_send`. The SYN packet was never sent.
- `[tcp.rx.scan.begin]` / TCP RX scan: Missing, deferred because SYN wasn't sent.
- `[tcp.synack.rx.ok]` / `[tcp.rst.rx.ok]`: **Missing.**
- `[tcp.synack.rx.missing]`: **True** (implicit, didn't even send SYN).

### Host/TAP Packet Path Diagnosis
The TCP packet path never triggered because the gateway ARP resolution failed. 
The guest is hardcoded to use IP `10.0.2.15` and attempts to resolve the SLiRP gateway at `10.0.2.2` or `10.0.2.1`.
```
[arp.request.send] sha=52:54:00:12:34:56 spa=10.0.2.15 tpa=10.0.2.1 oper=1 sent=1 tdt=1
[arp.gateway.tx.post] attempt=1 target_ip=10.0.2.2 tx_dd=1
[arp.gateway.rx.reply] attempt=1 rounds=64 reply_seen=0 spa=10.0.2.2 tpa=10.0.2.15 mac=00:00:00:00:00:00
```
However, the host TAP interface is configured as `10.0.3.1/24` (per `TAP_TUN_HOST_CAPABILITY_FIX_GUIDE_V1`). The host does not respond to ARP requests for `10.0.2.1` or `10.0.2.2` on `tap0`, nor does the guest subnet overlap properly for direct routing without NAT.

Because ARP failed to resolve the gateway MAC address (`gw_mac=00:00:00:00:00:00`), the TCP stack halted (`reason=gateway_unknown_no_syn_send`), and no SYN was transmitted to the TAP interface.

We must align the guest IP configuration with the TAP network (`10.0.3.x`), or configure the host TAP interface to respond to the `10.0.2.x` subnet. No TCP fixes can be attempted until this IP/ARP regression is resolved.
