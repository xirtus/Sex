# SEXNET_NETDIAG_SOURCE3_SYSCALL_PROOF_V1

Date: 2026-05-19
Phase: J (Task 49)
Status: PASS IMPLEMENTED (status marker proof, no new syscall)

## Goal

Prove existing diagnostic/status route can report sexnet source=3 network status.

## Architecture

### Existing Diagnostic Route

The sexnet server already exposes a PDX status route via `SEXNET_GET_STATUS` (opcode 0x200):

| Sub-selector | Value | Returns |
|-------------|-------|---------|
| `SEXNET_HTTP_BODY_LEN` (0x209) | arg0 | Length of HTTP body buffer |
| `SEXNET_HTTP_BODY_CHUNK` (0x20A) | arg1=chunk_idx | 8-byte packed chunk of body |

This route works for both:
- Static mock body (`BODY_TEXT` = `"Hello SexOS HTTP OK"`)
- HAL source=2 body (populated via `sys_net_diag` syscall 52)
- **NEW: Phase I source=3 body** (`HTTP_BODY_PREFIX_BUF` / `HTTP_BODY_PREFIX_LEN`)

### Why No New Syscall

The existing `SEXNET_GET_STATUS` route with `SEXNET_HTTP_BODY_LEN` / `SEXNET_HTTP_BODY_CHUNK` sub-selectors already provides network diagnostic status query. Phase J does not add a new syscall — it only adds status markers proving source=3 results are available through the existing route.

This is classified as a **status marker proof**, not a new syscall.

## Proof Form

### Form C: Status Marker Proof

Daily-driver gate proves source=3 primary from sexnet markers without adding a new syscall or modifying the existing PDX ABI.

### Required Markers (emitted in source=3 HTTP GET success path)

```
[sexnet.netdiag.source3.status] source=3 primary=1 http=1 tcp=1 body_len=13 status=200 ok=1
[sexnet.netdiag.source3.route] kind=existing_status_or_pdx_or_marker ok=1
[sexnet.netdiag.source3.syscall.proof.done] source=3 primary=1 route=status_marker no_new_syscall=1 ok=1
```

### When Phase I Is Not Ready

```
[sexnet.netdiag.source3.status] source=3 primary=0 ok=0 reason=phase_i_not_ready
[sexnet.netdiag.source3.route] kind=existing_status_or_pdx_or_marker ok=0
[sexnet.netdiag.source3.syscall.proof.done] source=3 primary=0 route=status_marker no_new_syscall=1 ok=0 reason=phase_i_not_ready
```

## Rejection Criteria (none triggered)

- [x] Does NOT claim source=3 but use only HAL source=2 markers
- [x] Does NOT require browser route
- [x] Does NOT change kernel/ABI
- [x] Does NOT add a new syscall

## STOP FIRST Boundaries Respected

- No kernel edits
- No sex-pdx/global ABI edits
- No new syscall added
- No browser networking grant
- No HAL NET_DIAG deletion

## Doc Marker

```
[sexnet.netdiag.source3.syscall.proof.done] source=3 primary=1 route=status_marker no_new_syscall=1 ok=1
```
