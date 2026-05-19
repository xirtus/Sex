# SEXNET_RX_TX_DESCRIPTOR_REUSE_PROOF_V1

Date: 2026-05-19
Branch: master
Task: 63 — Phase M RX/TX descriptor reuse proof

## Goal

Prove that TX and RX descriptors are safely reused across repeated source3 HTTP fetch iterations.

## Method

The multi-fetch loop uses:
- **TX descriptor**: Always slot 7 (desc index 7), same buffer (TX_PERM_FRAME_VA)
- **RX descriptors**: All 8 slots, scanned by poll, cleared on each iteration

### TX Reuse
1. Each iteration builds new ETH+IPv4+TCP+HTTP headers in the same TX frame buffer
2. Publishes via TDT tail write (slot 8 → wraps to 0 on 8-entry ring)
3. Polls DD bit on descriptor 7
4. After DD=1, hardware has consumed the descriptor
5. Next iteration overwrites buffer and publishes again
6. Safe because each TX is atomic: build→publish→poll→DD_consumed

### RX Reuse
1. Each iteration scans all 8 RX descriptors
2. On each descriptor with DD=1: copies payload, clears status byte, clears length, writes RDT
3. Hardware reclaims cleared descriptors for new frames
4. Next iteration's RX poll sees fresh descriptors
5. Bounded ring wrap: 8-entry ring, indexes modulo 8

## Markers

```
[sexnet.descriptor.reuse.tx] iter=0 slot=7 dd=1 tdt=8 ok=1
[sexnet.descriptor.reuse.rx] iter=0 slot=0 bytes=71 status_dd=1 cleared=1 ok=1
[sexnet.descriptor.reuse.tx] iter=1 slot=7 dd=1 tdt=8 ok=1
[sexnet.descriptor.reuse.rx] iter=1 slot=0 bytes=71 status_dd=1 cleared=1 ok=1
[sexnet.descriptor.reuse.tx] iter=2 slot=7 dd=1 tdt=8 ok=1
[sexnet.descriptor.reuse.rx] iter=2 slot=0 bytes=71 status_dd=1 cleared=1 ok=1
[sexnet.descriptor.reuse.proof.done] tx_reuse=3 rx_reuse=3 ok=1
```

## Rules

- Must not corrupt existing TX/RX proof lanes (descriptor slots 0-6 for other protocols preserved).
- Must not assume unbounded ring (8-entry ring, wrap-safe).
- Must wrap ring indexes safely (modulo 8).
- If RX reuse not exercised due environment, SKIP RX reuse honestly but TX reuse must be proven if multi-fetch sends.
- No ring architecture redesign.

## Classification

PASS IMPLEMENTED when tx_reuse=N rx_reuse=N (N=3) with all dd=1 and cleared=1.

If environment-limited: TX PASS / RX SKIP is acceptable (documented split).
