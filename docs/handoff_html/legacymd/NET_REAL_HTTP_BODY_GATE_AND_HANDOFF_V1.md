# NET_REAL_HTTP_BODY_GATE_AND_HANDOFF_V1

## Scope
Gate + handoff only.

No feature work, protocol work, syscall work, parser work, network-behavior changes, or refactors.

## Gate Added
- `net_real_http_body_prefix` in `scripts/daily_driver_master_gate.sh`

PASS requires all of the following runtime markers:
- `[net.diag.body.capture] bytes=64 cap=64 ok=1 source=real`
- `[sexnet.dynamic_body.set] len=64 source=2 ok=1`
- `[sexnet.body_text.len] len=64`
- `[browser.body.len.recv] len=64`
- `[browser.body.chunk.recv] idx=0 bytes=8`
- `[browser.body.chunk.recv] idx=1 bytes=8`
- `[browser.body.chunk.recv] idx=2 bytes=8`
- `[browser.body.chunk.recv] idx=3 bytes=8`
- `[browser.body.chunk.recv] idx=4 bytes=8`
- `[browser.body.chunk.recv] idx=5 bytes=8`
- `[browser.body.chunk.recv] idx=6 bytes=8`
- `[browser.body.chunk.recv] idx=7 bytes=8`
- `[browser.body.text.set] live=1 len=64`
- `[browser.body.render.done]`

If this TAP/host lane is not enabled in a given boot, the gate remains `SKIP` under existing daily-driver convention.

## Exact Proof Chain
1. Real host HTTP response is observed in kernel diagnostics.
2. Kernel captures a bounded 64-byte body prefix: `bytes=64 cap=64`.
3. Syscall 52 body selectors are used to feed userland body reads.
4. `sexnet` sets dynamic body text (`source=2`, real lane) with `len=64`.
5. Browser receives `len=64` and then 8 async scalar PDX chunks of 8 bytes each (`idx=0..7`).
6. Browser sets live body text and render completion marker fires.

## Semantics and Boundaries
- `source=real` means TAP + host HTTP server path.
- `source=mock` is not sufficient for `net_real_http_body_prefix`.
- Capture remains intentionally bounded to 64 bytes for this gate.
- Under `NETWORK_PROOF_CONTAINMENT_V1`, PCI HAL remains diagnostic-only.
- `sexnet`/browser body receive path is scalar async PDX chunks only (no pointer-copy/shared-memory transport).
- `sexdisplay` remains the sole framebuffer writer.

## Future Direction (Non-Goal Here)
Move NIC/TCP/HTTP ownership into `sexnet` or a dedicated driver service; do not expand HAL responsibilities.
