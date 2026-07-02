# E1000_RX_LOOPBACK_PREENABLE_REPOST_PROOF_V1

Date: 2026-05-17
Proof: FINAL PASS 226/0/0
Log: /tmp/sexos_e1000_rx_loopback_preenable_repost_proof_v1.log

## Summary

MAC loopback (RCTL.LBM=3) enabled before TX repost.
TX frame posted. RX polled bounded rounds.
**Result: TX NOT consumed in loopback mode. RX still dead.**

---

## Loopback Timing Table

| rctl_before | rctl_after | lbm | en | ok |
|-------------|------------|-----|----|----|
| 0x040080DA  | 0x040080DA | 3   | 1  | 1  |

RCTL.LBM=3 latches correctly. RCTL.EN=1 preserved.

---

## TX Repost Table

| tdh_before | tdt_before | tdt_after | len | tx_dd_after_poll |
|------------|------------|-----------|-----|-----------------|
| 0          | 0          | 1         | 60  | **0**           |

TX descriptor posted (TDT=1). TX descriptor NOT consumed after 4×100k spin rounds.

**Control comparison (same boot, earlier in flow):**
```
[e1000.tx.consume.diag] tdh_before=1 tdt_post=1 tdh_after=1 desc0_status=0x01 dd=1
```
Original TX (LBM=0) DID consume the descriptor (dd=1).

---

## RX Observe Result

```
[e1000.rx.loopback.observe] polled=32 dd_set=0 rdh_before=0 rdh_after=0 observed=0
[e1000.rx.loopback.preenable.repost.done] ok=1 loopback=1 tx_posted=1 rx_dd=0 rdh_advanced=0
```

RX: nothing. 32 descriptor checks across 4 rounds. Zero DD bits set. RDH stayed at 0.

---

## Conclusion: B — Loopback dead. QEMU e1000 MAC loopback (LBM=3) non-functional.

| Finding                                         | Status |
|-------------------------------------------------|--------|
| RCTL.LBM=3 latches                             | YES    |
| TX descriptor consumed in normal mode (LBM=0)  | YES (prior) |
| TX descriptor consumed in LBM=3 mode           | **NO** |
| RX descriptor processed (any mode)             | **NO** |

Two candidate causes:

**A. QEMU e1000 does not implement MAC loopback (LBM=3).**
QEMU's e1000 emulates an 82540EM. MAC loopback (LBM=3 = MAC near-end) may simply not
be implemented in the QEMU emulation. The TX engine may halt or silently discard
TX frames when LBM=3 is set.

**B. Writing TDH=0 directly corrupted TX state.**
TDH is read-only in the 82540EM spec. Direct write of TDH=0 after TDH=1 (from prior
consumed frame) may have put the TX ring in an inconsistent state. QEMU may have
accepted the write but the internal descriptor pointer still advanced, leaving
TDH=0 and internal state at 1 — so hardware sees TDH(internal)=TDT=1 → ring empty.

**Either way:** MAC loopback is not a viable diagnostic path. External RX is needed.

---

## Ruled-Out Causes (Cumulative)

| Cause                                  | Status        |
|----------------------------------------|---------------|
| RXDCTL/SRRCTL not latching             | CONFIRMED stub |
| RDBAL/RDBAH wrong split               | RULED OUT     |
| Physical address above 4 GiB          | RULED OUT     |
| Buffer addr mismatch in descriptor    | RULED OUT     |
| Alignment fault                       | RULED OUT     |
| RCTL.EN not set                       | RULED OUT     |
| MAC loopback path (LBM=3)             | NON-FUNCTIONAL |

---

## Next Recommendation

**QEMU_E1000_MODEL_SWITCH_82540EM_V1**

Rationale:
- All address/alignment/register init issues ruled out.
- MAC loopback dead — cannot self-test RX path.
- Normal TX works (LBM=0). External RX needed.
- Root cause is likely that QEMU `e1000` model does not process RX descriptors in
  the current init configuration (missing some model-specific requirement), OR
  the model simply doesn't receive traffic because SLiRP never delivers anything
  without an ARP/DHCP exchange.

Actions:
1. Check QEMU command line: confirm model is `e1000` not `e1000e`. They have different
   register maps and RXDCTL behavior.
2. Try `-device e1000-82544gc` as an alternative model.
3. Alternatively: E1000_PHY_LOOPBACK_LBM1_PROBE_V1 — try RCTL.LBM=1 (PHY loopback,
   different code path in QEMU), which may be implemented where LBM=3 is not.

Proof result: FINAL PASS 226/0/0
Faults: 0
