# HAL_NET_DIAG_SOURCE2_LEGACY_HANDOFF_V1

Date: 2026-05-19
Branch: master
Phase: L, Task 59

## Purpose

Formal handoff documenting all source=2/HAL diagnostics still present,
their exact retention reasons, limitations, rollback instructions, and
STOP boundaries. This handoff freezes source=2 as legacy/fallback without
deleting any code.

## Source=2/HAL Diagnostics Still Present

| Diagnostic | Location | Retention Reason |
|-----------|----------|-----------------|
| HAL TCP probe | kernel/src/hal/pci.rs ~line 2730-3510 | Legacy TCP connection test; disabled by SEXOS_HAL_TCP_PROBE=0 |
| HAL HTTP status store | kernel/src/hal/pci.rs lines 18-22 (NET_DIAG_HTTP_*) | Fallback HTTP body capture |
| HAL HTTP body capture | kernel/src/hal/pci.rs lines 3378-3384 | Legacy body buffer (64 bytes) |
| HAL DNS A-record resolution | kernel/src/hal/pci.rs ~line 3090-3300 | DNS review-only (Phase F); needed for future source3 DNS migration |
| sys_net_diag() syscall | kernel/src/hal/pci.rs lines 18-47, servers/sexnet/src/main.rs lines 67, 481-510 | Allows sexnet to read HAL diagnostic state for fallback |
| HAL NIC enumeration | kernel/src/hal/pci.rs e1000 probe | Hardware bringup and fallback NIC init |
| [sexnet.dynamic_body.set] source=2 marker | servers/sexnet/src/main.rs line 529 | Legacy fallback body set from HAL diagnostics |

## Exact Reason Each Remains

### Fallback (TCP probe, HTTP status/body, dynamic_body.set)
If sexnet source=3 is unavailable (no peer, no NIC, compile issue),
the HAL diagnostic path can provide a degraded network diagnostic.
This fallback is only active when `SEXOS_HAL_TCP_PROBE` is not set to 0.

### DNS Review-Only (HAL DNS A-record)
Phase F DNS is implemented only in HAL/source=2. Future source=3 DNS
migration will reference this implementation. Review-only: no runtime
DNS for browser until Phase L+/M.

### Hardware Bringup (HAL NIC enumeration)
The HAL e1000 probe is the primary mechanism for NIC BAR discovery,
ring initialization, and register programming. Sexnet consumes this
infrastructure. Retained as the NIC driver foundation.

### Rollback (all source=2 diagnostics)
If source=3 encounters a regression, re-enabling HAL TCP probe
(`SEXOS_HAL_TCP_PROBE=1`) provides a known-working diagnostic path.

## Exact Source=2 Limitations

| Limitation | Detail |
|-----------|--------|
| NOT browser primary | Browser uses source=3 (Phase K) or source=1 (offline) |
| NOT HTTP primary | HTTP truth is source=3 (Phase I) |
| NOT body truth | source=3 body is authoritative |
| NOT final network truth | source=3 is the primary network diagnostic |
| DNS only via HAL | source=2 DNS is review-only; no source3 DNS yet |
| 64-byte body cap | HAL body buffer is 64 bytes; source3 extends this |
| No browser route | HAL never routes to browser; sexnet + PDX do |

## Rollback Instructions

To re-enable HAL TCP probe as primary diagnostic:

1. Set `SEXOS_HAL_TCP_PROBE=1` (or unset it, since default is enabled).
2. Remove or disable the Phase L explicit source3 profile.
3. Run daily proof without `SEXNET_PHASE_I_HTTP_PROOF=1`.
4. Verify `[hal.tcp.probe.gate] enabled=1` in log.

Note: This should only be done for debugging or when source=3 is
known-unavailable. In normal operation, source=3 should remain primary.

## STOP Boundaries

- DELETE/FREEZE HAL runtime code: ONLY in later phase after Phase M (reliability)
  and Phase N (real hardware) are complete and safe.
- NO HAL deletion in Phase L.
- NO DNS source3 claim in Phase L.
- NO syscall ABI changes.
- NO sex-pdx ABI changes.
- NO kernel HAL behavior changes.

## Doc Marker

[hal.netdiag.source2.legacy.handoff.done] ok=1
[hal.netdiag.source2.legacy] dns=review_only http=fallback primary=0 ok=1
