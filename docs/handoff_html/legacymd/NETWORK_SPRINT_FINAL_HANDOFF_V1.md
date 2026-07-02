# NETWORK_SPRINT_FINAL_HANDOFF_V1

## A. Sprint result
- Mission: `NETWORK_SPRINT_FINAL_HANDOFF_V1`
- Result: `PASS`
- Final proven state: TAP/e1000e backend reaches host HTTP server and real HTTP status/body prefix reaches browser render path.
- Final gate: `net_real_http_body_prefix PASS real(2)->sexnet len64->8x8 chunks->render done`
- Final proof: `FINAL: PASS (247 gates proved, 16 skipped, 0 faults)`

## B. Final proof command
Use this bounded diagnostic proof lane with TAP + e1000e:

```bash
QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
./scripts/run_daily_driver_proof.sh /tmp/sexos_network_sprint_final_handoff_v1.log
```

Expected gate/proof lines in the resulting log:
- `net_real_http_body_prefix PASS real(2)->sexnet len64->8x8 chunks->render done`
- `FINAL: PASS (247 gates proved, 16 skipped, 0 faults)`

## C. Final marker evidence
Required markers for the real body-prefix gate:
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

## D. Route architecture
Final route proven in this sprint:
- `host Python HTTP server`
- `-> TAP/e1000e`
- `-> bounded PCI HAL diagnostic HTTP proof`
- `-> kernel NET_DIAG status/body atomics`
- `-> syscall 52 status/body selectors`
- `-> sexnet dynamic status/body`
- `-> async packed PDX chunks`
- `-> Kaleidoscope/browser receive`
- `-> sexdisplay surface render path`

Architecture constraints retained:
- PCI HAL remains diagnostic-only.
- `sexdisplay` remains the sole framebuffer writer.
- Browser body delivery is scalar async PDX chunks only.
- No pointer-copy or shared-memory body transfer used.

## E. TAP/host requirements
Host preconditions for this proof lane:
- `/dev/net/tun` exists.
- `tap0` exists and is admin `UP`.
- `tap0` has `10.0.2.2/24`.
- Host server runs on TAP host IP:

```bash
python3 -u -m http.server 18080 --bind 10.0.2.2
```

Runtime env expected by proof:
- `QEMU_NET_BACKEND=tap`
- `QEMU_NET_MODEL=e1000e`
- `QEMU_TAP_IFNAME=tap0`
- `ENABLE_QEMU_USERNET_E1000=1`

## F. What is real vs mock
Truth semantics:
- `source=2` means real TAP HTTP path.
- `source=1` means mock fallback.
- `source=mock` is not sufficient for the real body gate.

Honesty boundary:
- This is a bounded diagnostic proof of end-to-end status/body transport and render.
- This is not production TCP/HTTP service ownership.

## G. Files/features changed during sprint
Feature-level sprint outcomes now proven:
- TAP/e1000e real ingress path used for browser HTTP body prefix proof.
- `sexnet` dynamic status/body feed path to browser proven with `len=64` body prefix.
- Browser async 8x8 packed chunk receive/render path proven (`idx=0..7`).
- Final scoreline reached with zero faults: `FINAL: PASS (247 gates proved, 16 skipped, 0 faults)`.

Scope note:
- This handoff is docs-only and does not add new code changes.

## H. Boundaries / STOP FIRST rules
- STOP FIRST if anyone proposes expanding PCI HAL into DNS/TLS/browser policy/general HTTP ownership.
- STOP FIRST if a claim says "real HTTP body proved" without `source=2` and the required markers in Section C.
- STOP FIRST if mock markers (`source=1` or `source=mock`) are used to satisfy `net_real_http_body_prefix`.
- STOP FIRST before introducing shared-memory or pointer-copy body transport.
- STOP FIRST before adding any framebuffer writer outside `sexdisplay`.
- STOP FIRST before changing kernel/ABI ownership boundaries without explicit mission scope.

Long-term ownership boundary:
- Real TCP/HTTP ownership should migrate to `sexnet` or a dedicated NIC/driver service, not remain in PCI HAL diagnostics.

## I. Next recommended missions
1. `SEXNET_HTTP_OWNERSHIP_MIGRATION_PLAN_V2`
- Plan real TCP/HTTP ownership outside PCI HAL.
2. `TAP_HOST_SETUP_STABILITY_V1`
- Make TAP setup less fragile across sessions.
3. `BROWSER_REMOTE_BODY_LAYOUT_POLISH_V1`
- Display the 64-byte prefix more cleanly, no protocol changes.
4. `SEXNET_SOURCE_TRUTH_DASHBOARD_V1`
- Expose real/mock/source status in UI.
5. `NETWORK_PROOF_COMMIT_AUDIT_V1`
- Verify all docs/scripts/code committed cleanly.
