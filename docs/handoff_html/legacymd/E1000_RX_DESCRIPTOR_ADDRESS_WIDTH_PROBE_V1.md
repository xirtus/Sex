# E1000_RX_DESCRIPTOR_ADDRESS_WIDTH_PROBE_V1

Date: 2026-05-17
Commit baseline: post-896ae00 (bank/persistence/ownership probe added, not yet committed)
Proof: FINAL PASS 226/0/0
Log: /tmp/sexos_e1000_rx_descriptor_address_width_probe_v1.log

## Summary

Address-width and base-address probe added to `kernel/src/hal/pci.rs`.
No ABI, protocol, or grant changes. Build clean. Gates stable at 226/0/0.

**Result: address_width_ok=1 — address programming is NOT the blocker.**

---

## Ring Base Address Table

| Field         | Value                  |
|---------------|------------------------|
| rx_phys       | 0x000000001F86C000     |
| RDBAL readback| 0x1F86C000             |
| RDBAH readback| 0x00000000             |
| Reconstructed | 0x000000001F86C000     |
| below_4g      | YES                    |
| match         | YES (exact)            |
| align_16      | YES                    |
| align_4k      | YES                    |

---

## Buffer Address Table

| Field         | Value                  |
|---------------|------------------------|
| desc0_buf     | 0x00000000102AB000     |
| buf0_phys     | 0x00000000102AB000     |
| below_4g      | YES                    |
| match         | YES (exact)            |
| align_16      | YES                    |
| align_2048    | YES                    |

---

## Alignment Table

| Resource      | Alignment  | Required | Status |
|---------------|-----------|----------|--------|
| RX ring       | 4096-byte | 16-byte min (128+ preferred) | PASS |
| RX buffer[0]  | 2048-byte | 16-byte min | PASS |

---

## Address-Width Conclusion

**Address programming is correct.** All checks pass:
- Ring physical address is below 4 GiB → RDBAH correctly 0.
- Reconstructed address from RDBAL/RDBAH matches `rx_p` exactly.
- Descriptor[0] buffer pointer in ring memory matches `pkt_pages[0]` exactly.
- All addresses well-aligned.

**This rules out:**
- RDBAL/RDBAH split error
- High-memory DMA issue (>4 GiB)
- Buffer address corruption in descriptor
- Alignment fault

**RX still dead** despite correct addresses: `rdh=0, rdt=7, dd=0, packets=0`.

---

## Accumulated Ruled-Out Causes

| Cause                                  | Status    |
|----------------------------------------|-----------|
| RXDCTL.ENABLE not latching             | CONFIRMED stub |
| SRRCTL not latching                    | CONFIRMED stub |
| RDBAL/RDBAH wrong split                | RULED OUT |
| Physical address above 4 GiB          | RULED OUT |
| Descriptor buffer addr mismatch        | RULED OUT |
| Alignment issue                        | RULED OUT |
| RCTL.EN not set                        | RULED OUT (prior) |
| Ring register programming wrong        | RULED OUT (prior) |

---

## Next Recommendation

**E1000_RX_DESCRIPTOR_FORMAT_VARIANT_PROBE_V1**

The only remaining candidates:

1. **RCTL buffer-size encoding** — RCTL.BSIZE (bits 16-17) = 00 → 2048 bytes.
   RCTL.BSEX (bit 25) = 0. With 2048-byte buffers this is correct for 82540EM.
   Probe: try RCTL variants with explicit BSIZE values and verify they match buffer allocation.

2. **MAC loopback path** — RCTL.LBM=3 is set in the poll loop after TX test frame is sent.
   The TX frame is sent BEFORE loopback is enabled. If we enable loopback first, TX frames
   should loop back to RX — this would prove whether the RX path processes descriptors at all.
   Probe: enable RCTL.LBM=3, send one minimal TX frame, poll RX with bounded rounds.
   This is the most direct live test of the RX descriptor processing path.

3. **Descriptor legacy vs extended format** — QEMU e1000 may expect extended (e1000e)
   descriptor format in some configurations. Verify with explicit field layout check.

4. **QEMU e1000 model RX state machine** — If LBM loopback test fails, the QEMU
   e1000 model's RX descriptor processing may be broken for this configuration.
   Next step would be QEMU_E1000_MODEL_SWITCH_82540EM_V1 (change `-device` model string).

Proof result: FINAL PASS 226/0/0
Faults: 0
