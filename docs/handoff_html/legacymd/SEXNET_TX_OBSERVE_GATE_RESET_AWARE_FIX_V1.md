# SEXNET_TX_OBSERVE_GATE_RESET_AWARE_FIX_V1

## Problem

The e1000e `CTRL.RST` reset (introduced in `d5116d9`) leaves the NIC TX registers in
post-reset defaults (TCTL=0, EN=0). The TX observe save/restore mechanism correctly
saves these zeros and restores them after the observe window, but:

1. **Code**: The restore validation in `main.rs` hardcoded `tx_tctl_en_restored == 1`,
   which fails when the saved state has EN=0 (post-reset).

2. **Gate**: The `sexnet_nic_tx_frame_observe` gate in `daily_driver_master_gate.sh`
   required `tctl_en=1` in the restore marker for PASS/SKIP, and treated `tctl_en=0`
   as FAIL.

After the reset, both the code validation and the gate falsely reported failure:
`tx_restore_ok=0` in the code, and FAIL in the gate scan.

## Fix

### `servers/sexnet/src/main.rs` — TX observe restore validation

1. Extract `tx_tctl_en_orig` from saved `tx_tctl_orig` to capture original enable state
2. Compare `tx_tctl_en_restored == tx_tctl_en_orig` instead of hardcoded `== 1`
3. Add `tctl_en_orig` and `tctl_en` to both save and restore serial markers

### `scripts/daily_driver_master_gate.sh` — Gate reset-awareness

1. PASS/SKIP conditions check `ok=1` on restore marker instead of `tctl_en=1`
2. FAIL condition checks `ok=0` on restore marker instead of `tctl_en=0`
3. Marker regex accepts both `tctl_en_orig=0 tctl_en=0` and `tctl_en_orig=1 tctl_en=1`

## Proof Results (2026-05-19)

### TX Observe Markers (all ok=1)
```
[sexnet.nic.tx.observe.alloc] desc_phys=0x00000000102C6000 frame_phys=0x00000000102C7000 ok=1
[sexnet.nic.tx.observe.frame.write] ethertype=0x88B5 len=60 ok=1
[sexnet.nic.tx.observe.desc.write] len=60 cmd=0x0B sta=0 ok=1
[sexnet.nic.tx.observe.ring.save] tctl=0x00040100 tctl_en=0 tdbal=0x102AA000 tdlen=128 tdt=1 ok=1
[sexnet.nic.tx.observe.ring.program] tdbal=0x102C6000 tdlen=128 tdt=0 tctl=0x00040102 ok=1
[sexnet.nic.tx.observe.post] tdt=1 ok=1
[sexnet.nic.tx.observe.poll.begin] max_iters=50000000
[sexnet.nic.tx.observe.poll.done] dd_set=1 desc_idx=0 ok=1
[sexnet.nic.tx.observe.ring.restore] tctl_orig=0x00040100 tctl_en_orig=0 tctl_en=0 tdbal=0x102AA000 ok=1
[sexnet.nic.tx.observe.proof.done] dd_set=1 ok=1
```

Key: `tctl_en_orig=0` (post-reset default), `tctl_en=0` (restored correctly), `ok=1`.

### TX Permanent
```
[sexnet.nic.tx.permanent.poll.done] dd_set=1 desc_idx=0 ok=1
[sexnet.nic.tx.permanent.claim] owner=2 ring_ok=1 ok=1
[sexnet.nic.tx.permanent.full] rx_owner=3 tx_owner=3 full_ok=1
```

### TCP Handshake
```
[sexnet.tcp.syn.tx.proof.done] tx=1 tx_dd=1 ok=1
[sexnet.tcp.handshake.state] state=FAILED_RST ok=1  (RST from no-peer, honest)
```

### Gate Scan
```
sexnet_nic_tx_frame_observe   PASS   tx observe/restore proof (reset-aware, DD proven)
sexnet_e1000e_reset_rx        PASS   e1000e CTRL.RST -> RX ownership transition proof
sexnet_tcp_handshake          PASS   Phase G: TCP SYN TX proven, RST observed (honest)
sexnet_tcp_payload            PASS   Phase H: TCP payload guard proven, honest block (env-limited)
PASS gates: 249  FAIL: 0  SKIP: 48  FAULTS: 0  FINAL: PASS
```

## STOP Boundaries

- No kernel edits
- No HTTP/TCP/browser/socket code changes
- No sex-pdx/global ABI edits
- No DMA memory ownership model change
- No scheduler/PKRU/time changes
- No NIC driver rewrite

## Classification

**PASS IMPLEMENTED** — Code fix + gate update. TX observe now correctly handles both
pre-reset (HAL EN=1) and post-reset (default EN=0) TCTL states. 249 gates PASS, 0 faults.
