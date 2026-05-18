# SEXNET_ARP_REPLY_HOST_OBSERVE_PROOF_V1

## A. Purpose
Provide a host-side proof that the host can observe ARP reply behavior associated with sexnet on the TAP path.

## B. Preconditions
- TAP backend is required.
- Host has a TAP interface (default `tap0`) configured and usable by QEMU.
- `arping` is available on host.
- This mission is host-side observation only.
- No SexOS code changes are part of this mission.

## C. Run sequence
Run both terminals concurrently.

Terminal 1:
```bash
QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
./scripts/run_daily_driver_proof.sh /tmp/sexnet_arp_gate.log
```

Terminal 2:
```bash
QEMU_TAP_IFNAME=tap0 ./scripts/host_arp_reply_observe_probe.sh
```

## D. PASS/SKIP/FAIL criteria
- PASS:
  - `arping` output includes `Unicast reply` or `bytes from`, or
  - `ip neigh show 10.0.2.15 dev tap0` includes `lladdr`.
- SKIP:
  - QEMU network backend is slirp/user backend instead of TAP.
  - TAP interface is unavailable by environment policy.
- FAIL:
  - TAP path is intended and available, but no ARP reply indicators are observed in the probe window.

## E. Log path
- `/tmp/sexnet_arp_host_observe.log`

## F. What this proves
- Host-side observation sees ARP-reply evidence for guest IP `10.0.2.15` on TAP path.
- Wire-visible PASS in this lane required ARP TX on `slot=1` with `tdt=2`, then L2 reuse on `slot=2` with `tdt=3`.
- Prior ARP `slot=2` / `tdt=3` ordering could still DD-complete without a host-visible ARP reply, so slot ordering is part of the proof contract.

## G. What this does not prove
- Does not prove IP/TCP/HTTP connectivity.
- Does not prove higher-layer protocol correctness.
- Does not alter or validate SexOS internal protocol logic.
- Does not prove ARP cache behavior.
- Uses poll-driven TX/RX markers; IRQ behavior is not part of this proof.
- Browser/`NET_DIAG` source path is unchanged by this mission.

## H. STOP FIRST rules
Stop immediately if any of the following becomes necessary:
- Any forbidden file appears necessary.
- Script requires `tcpdump`.
- Any attempt to edit SexOS source is proposed.
- Any attempt to add or modify daily-driver gate behavior is proposed.

## I. Next missions
- Run TAP host ARP observe proof repeatedly across boot timing windows.
- Correlate host-observed ARP reply timing with existing sexnet runtime markers (without changing gates).
- Expand host-side observability docs for TAP prerequisites and failure taxonomy.
