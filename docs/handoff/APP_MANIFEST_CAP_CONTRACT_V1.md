# APP_MANIFEST_CAP_CONTRACT_V1

**Status:** LOCKED ✅  
**Date:** 2026-05-06  
**Prerequisite:** APP_SURFACE_LAUNCH_CONTRACT_V1 (passes)  
**Files changed:** 2 (+125 lib.rs, +95 main.rs / net +220 lines)

---

## Contract

An app-like PD can declare bounded identity and requested capabilities via a packed manifest in `OP_APP_SURFACE_REQ` (0xFA). The shell unpacks, validates, and rejects any unknown or denied capability. No PD ever gets raw framebuffer access, shell policy ownership, or unrestricted PDX access through this contract.

### Manifest Shape

```rust
pub struct AppManifest {
    pub surface_id: u64,          // >= 200
    pub title_id: u64,            // non-zero, opaque
    pub app_id: u16,              // bounded 16-bit discriminator
    pub capabilities: AppCapabilityBits,
}

pub struct AppCapabilityBits(u8);
```

Packed into PDX message:

| Arg    | Field                                |
|--------|--------------------------------------|
| arg0   | `surface_id` (full u64)              |
| arg1   | `title_id` (full u64)                |
| arg2   | bits 0-7: capabilities               |
|        | bits 8-23: app_id                    |
|        | bits 24-55: reserved (must be 0)     |
|        | bits 56-63: version (must be 0)      |

### Known Capability Bits

| Bit | Name       | Description                          |
|-----|------------|--------------------------------------|
| 0   | BELL       | May send events to Bell notification |
| 1   | SEXFILES   | May access SexFiles VFS storage      |
| 2-7| Reserved   | Rejected as unknown                  |

### Validation Rules

1. **Manifest unpack**: version != 0 → reject. Reserved bits non-zero → reject.
2. **surface_id**: zero → reject. `< 200` (OS reserved) → reject.
3. **title_id**: zero → reject.
4. **Already registered** in lifecycle → reject.
5. **Capability bits**: any bit outside `{BELL, SEXFILES}` → reject (deny unknown).
6. **Display/framebuffer ownership**: NOT representable as a capability bit → deny-by-default.
7. **Shell policy ownership**: NOT representable as a capability bit → deny-by-default.
8. **No free frame slot** → reject.

### Proof Markers (gated by `SEXOS_APP_SURFACE_REQ_PROOF=1`)

8 stages run at boot before the main listen loop:

| Stage | Call | Expected | Marker |
|-------|------|----------|--------|
| 0 | `handle_app_surface_req(300, 42, 0, 0)` | accepted (valid, no caps) | `[shell.app_surface.proof] stage=0 accepted=true` |
| 1 | `handle_app_surface_req(0, 42, 0, 0)` | rejected (zero sid) | `[shell.app_surface.proof] stage=1 accepted=false` |
| 2 | `handle_app_surface_req(301, 0, 0, 0)` | rejected (zero title) | `[shell.app_surface.proof] stage=2 accepted=false` |
| 3 | `handle_app_surface_req(300, 99, 0, 0)` | rejected (duplicate sid) | `[shell.app_surface.proof] stage=3 accepted=false` |
| 4 | `handle_app_surface_req(302, 55, packed_bell, 0)` | accepted (Bell cap) | `[shell.app_surface.proof] stage=4 accepted=true` |
| 5 | `handle_app_surface_req(303, 56, packed_unknown, 0)` | rejected (unknown cap 0x80) | `[shell.app_surface.proof] stage=5 accepted=false` |
| 6 | `handle_app_surface_req(304, 57, bad_version, 0)` | rejected (bad version) | `[shell.app_surface.proof] stage=6 accepted=false` |
| 7 | `handle_app_surface_req(305, 58, bad_reserved, 0)` | rejected (reserved bits) | `[shell.app_surface.proof] stage=7 accepted=false` |

### Rejection Markers (runtime)

| Marker | Reason |
|--------|--------|
| `[shell.app_surface.reject] reason=manifest_invalid` | Manifest unpack failed |
| `[shell.app_surface.reject] reason=zero_surface_id` | surface_id == 0 |
| `[shell.app_surface.reject] reason=zero_title_id` | title_id == 0 |
| `[shell.app_surface.reject] reason=already_registered` | Duplicate surface_id |
| `[shell.app_surface.reject] reason=reserved_range` | surface_id < 200 |
| `[shell.app_surface.reject] reason=no_frame_slot` | No free frame |

### Acceptance Marker

`[shell.app_surface.accept] sid=X title_id=X frame=X caps=0xX app_id=X caller=X`

### Capability Log Marker (non-zero caps only)

`[shell.app_surface.capabilities] sid=X caps=0xX desc=bell app_id=X`

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/lib.rs` | Added `AppCapabilityBits` struct + `AppManifest` struct with pack/unpack |
| `servers/silk-shell/src/main.rs` | Updated `handle_app_surface_req` to accept arg2 manifest, added validation gates, extended proof to 8 stages, updated dispatch |

## Build / Runtime

- Build: PASS (0 errors, pre-existing warnings only)
- Proof build (`SEXOS_APP_SURFACE_REQ_PROOF=1`): PASS
- Full OS build: PASS (sexshop app pre-existing errors, unrelated)
- No kernel edits. No sex-pdx ABI changes. No renderer primitives.
- Feature-gated: proof is default-off, zero behavior change when env var unset.

## STOP FIRST Conditions NOT Triggered

- sex-pdx ABI change: **NOT NEEDED** — manifest is packed into existing PDX args
- kernel/process loader change: **NOT NEEDED** — validation is userland shell-only
- broad SDK design: **NOT SUGGESTED** — two types in silk-shell lib.rs
- renderer policy/app framebuffer write: **NOT ADDED** — manifest explicitly denies display ownership

## Remaining Risks

1. **No dynamic cap enforcement**: The manifest is a declaration-of-intent, not a capability grant. An app that declares `BELL` still needs a separate Bell PDX slot to actually send notifications. Future V2 should wire manifest caps to slot grants.
2. **No error reply on reject**: The caller (if any) gets no explicit error code. Same limitation as base APP_SURFACE_LAUNCH_CONTRACT_V1.
3. **No process loader wiring**: The manifest is only validated at surface request time. A future PD spawn path should also validate manifests.
