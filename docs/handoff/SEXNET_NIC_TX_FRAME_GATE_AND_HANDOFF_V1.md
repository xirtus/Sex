# SEXNET_NIC_TX_FRAME_GATE_AND_HANDOFF_V1

## A. Result
Added a narrow daily-driver gate `sexnet_nic_tx_frame_observe` in `scripts/daily_driver_master_gate.sh` for the proven temporary sexnet TX descriptor completion proof.

The gate is non-invasive for ordinary boots and stays `SKIP` when the proof lane is not enabled.

## B. Proof command / preconditions
Preferred proof lane:

```bash
QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
./scripts/run_daily_driver_proof.sh /tmp/sexnet_nic_tx_frame_gate_and_handoff_v1.log
```

Scan:

```bash
grep -E "sexnet_nic_tx_frame_observe|sexnet.nic.tx.observe|fault.kill|#PF|#GP|panic|KERNEL PANIC|FINAL:" \
/tmp/sexnet_nic_tx_frame_gate_and_handoff_v1.log | tail -360
```

## C. Marker evidence
Gate PASS requires:

- `[sexnet.nic.tx.observe.alloc] ... ok=1`
- `[sexnet.nic.tx.observe.frame.write] ethertype=0x88B5 len=60 ok=1`
- `[sexnet.nic.tx.observe.desc.write] len=60 cmd=0x0B sta=0 ok=1`
- `[sexnet.nic.tx.observe.ring.program] ... tdlen=128 ... ok=1`
- `[sexnet.nic.tx.observe.post] tdt=1 ok=1`
- `[sexnet.nic.tx.observe.poll.done] dd_set=1 desc_idx=0 ok=1`
- `[sexnet.nic.tx.observe.ring.restore] ... tctl_en=1 ... ok=1`
- `[sexnet.nic.tx.observe.proof.done] dd_set=1 ok=1`

## D. What was proven
- Temporary sexnet TX ring can be programmed.
- A raw 60-byte Ethernet frame can be posted via descriptor 0.
- NIC consumed the descriptor (DD observed).
- Original HAL TX ring state can be restored.
- No-fault proof lane was maintained.

## E. What was not proven
- Not permanent NIC ownership transfer.
- Not ARP/IP/TCP/HTTP protocol semantics.
- Not host/tcpdump visibility proof.
- Not IRQ-driven TX completion model.

## F. Architecture boundary
This is temporary TX observe/restore proof only.

PCI HAL diagnostic bridge remains preserved; this gate confirms descriptor-consumption and restoration evidence, not a production ownership or protocol transition.

## G. STOP FIRST rules
Stop first if any of these occur:

- Gate would hard-fail normal non-enabled/non-NIC boots.
- Marker names drift and require source renames.
- Code-file edits are required outside allowed scope.
- Restore marker shows `tctl_en=0` or proof marker shows `ok=0`.

## H. Next missions
1. `SEXNET_NIC_OWNERSHIP_STATE_MACHINE_PLAN_V1`
