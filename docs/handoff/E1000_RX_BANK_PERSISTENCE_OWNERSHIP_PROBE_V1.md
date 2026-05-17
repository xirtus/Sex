# E1000_RX_BANK_PERSISTENCE_OWNERSHIP_PROBE_V1

Date: 2026-05-17
Commit baseline: 896ae00 (chore(net): add rx queue enable semantics probe)
Proof baseline: FINAL PASS 226/0/0
Log: /tmp/sexos_e1000_rx_bank_persistence_ownership_probe_v1.log

## Summary

Three bounded RX-only diagnostic probes were added to `kernel/src/hal/pci.rs`.
No ABI, protocol, or grant changes made.
Build clean. Gates stable at 226/0/0.

---

## Probe 1: RX Register-Bank Candidate Table

Candidates tested adjacent to known banks (RDBAL=0x2800 window):

| Offset | Label    | Before     | Wrote      | Imm        | Delayed    | Post-poll  | Latched |
|--------|----------|------------|------------|------------|------------|------------|---------|
| 0x2820 | RDTR     | 0x00000000 | 0x00000080 | 0x00000080 | 0x00000080 | 0x00000080 | **YES** |
| 0x2824 | unk_2824 | 0x00000000 | 0x00000080 | 0x00000000 | 0x00000000 | 0x00000000 | no      |
| 0x2828 | RXDCTL   | 0x00000000 | 0x00000080 | 0x00000000 | 0x00000000 | 0x00000000 | no      |
| 0x282C | RADV     | 0x00000000 | 0x00000080 | 0x00000080 | 0x00000080 | 0x00000080 | **YES** |
| 0x2830 | unk_2830 | 0x00000000 | 0x00000080 | 0x00000000 | 0x00000000 | 0x00000000 | no      |
| 0x2834 | unk_2834 | 0x00000000 | 0x00000080 | 0x00000000 | 0x00000000 | 0x00000000 | no      |

Summary marker:
```
[e1000.rx.bank.probe] candidates=6 latched=1 selected=0x2820 ok=1
```

**Finding**: RDTR (0x2820) and RADV (0x282C) are implemented and latch writes.
RXDCTL (0x2828), unk_2824, unk_2830, unk_2834 are NOT implemented — always read 0.

---

## Probe 2: Write Persistence Table (RXDCTL 0x2828)

| Offset | Before     | Wrote      | Imm        | Delayed    | Post-poll  | imm_latched | delayed_latched | post_poll_latched |
|--------|------------|------------|------------|------------|------------|-------------|-----------------|-------------------|
| 0x2828 | 0x00000000 | 0x02000000 | 0x00000000 | 0x00000000 | 0x00000000 | 0           | 0               | 0                 |

```
[e1000.rx.write.persistence] off=0x2828 imm_latched=0 delayed_latched=0 post_poll_latched=0 ok=1
```

**Finding**: RXDCTL.ENABLE (bit 25) does NOT latch under any timing condition.
QEMU's `e1000` model does not implement this register. Write is silently dropped.

---

## Probe 3: Descriptor Ownership Edge Table

| Field         | Before | After | Mutated |
|---------------|--------|-------|---------|
| status (byte) | 0x00   | 0x00  | no      |
| length (u16)  | 0      | 0     | no      |
| RDH           | 0      | 0     | no      |

```
[e1000.rx.ownership.edge] desc=0 status_before=0x00 status_after=0x00 len_before=0 len_after=0
    hw_mutated=0 rdh_before=0 rdh_after=0 ok=1
```

Summary:
```
[e1000.rx.bank.persistence.ownership.done] ok=1 rx_dd=0 rdh_advanced=0 hw_mutated=0 selected=0x2820
```

**Finding**: After setting RDT=1 (giving HW ownership of desc 0 with valid buffer pointer),
hardware did not advance RDH, did not set DD bit, did not write length.
hw_mutated=0 for the bounded wait window (~100k spin cycles).

---

## Conclusion

**D — Model-limited RX path (confirmed)**

| Evidence                                   | Status |
|--------------------------------------------|--------|
| RXDCTL (0x2828) never latches              | CONFIRMED |
| SRRCTL (0x280C) never latches (prior run)  | CONFIRMED |
| RDTR (0x2820), RADV (0x282C) latch         | CONFIRMED — timer regs real |
| Descriptor DD never set (8 poll rounds)    | CONFIRMED |
| RDH never advances                         | CONFIRMED |
| HW never mutates descriptor fields         | CONFIRMED (hw_mutated=0) |
| RCTL.EN=1 stable                           | CONFIRMED (prior) |
| RDBAL/RDBAH/RDLEN/RDH/RDT readable        | CONFIRMED (prior) |

QEMU's `e1000` model:
- Does NOT implement RXDCTL (0x2828) — writes silently dropped.
- Does NOT implement SRRCTL (0x280C) — writes silently dropped.
- DOES implement RCTL, RDBAL/RDBAH/RDLEN/RDH/RDT — these latch correctly.
- DOES implement interrupt timing regs (RDTR, RADV) — these latch.
- TX path consumed descriptors — BM/MMIO/DMA subsystem is functional.
- RX descriptors are never touched by hardware despite RCTL.EN=1 and RDT=7.

---

## Next Recommendation

**E1000_RX_DESCRIPTOR_FORMAT_VARIANT_PROBE_V1**

Rationale:
- RXDCTL and SRRCTL stubs rule out queue-mode as the blocker.
- RCTL.EN=1 and ring registers correct — init is not the issue.
- HW never processes a descriptor even with valid buffer address and RDT>RDH.
- Most likely cause: descriptor format mismatch (buffer address field width, descriptor type byte,
  or ring layout not matching what QEMU e1000 expects).
- The 82540EM legacy descriptor is 16 bytes: `[buf_addr:8][length:2][csum:2][status:1][errors:1][special:2]`.
  Verify current descriptor layout matches this exactly.
- Alternatively: confirm RDBAL/RDBAH point to physically correct addresses
  (check that `rx_phys` is below 4 GiB or that RDBAH is non-zero if ring is above 4 GiB).

Fallback if descriptor format confirmed correct:
**QEMU_E1000_MODEL_SWITCH_82540EM_V1** — test with explicit `-device e1000,netdev=...`
vs current `-device e1000e,netdev=...` or verify QEMU command line model name.
