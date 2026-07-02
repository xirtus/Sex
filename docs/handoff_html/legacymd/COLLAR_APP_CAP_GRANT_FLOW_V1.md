# COLLAR_APP_CAP_GRANT_FLOW_V1 — Persisted Capability Grant/Revoke

**Date:** 2026-05-06
**Status:** Implemented, proof-gated
**Gate:** `SEXOS_COLLAR_APP_CAP_GRANT_PROOF=1`

---

## Grant/Revoke Route

```
AppManifest (capability_bits)
  → Collar auto-grant (silk-shell)
    → collar_auto_grant_from_manifest()  [in-memory CollarGrant]
    → collar_persist_cap_record_to_sexfiles()  [persisted SexFiles CapRecord]
      → pdx_call(SLOT_LINEN, OP_LINEN_GRANT_CAP)
        → Linen handle_linen_grant_cap()
          → pdx_storage_sync(OP_RAMFS_GRANT_CAP)
            → RamFS.grant_cap_by_object_id()  [creates CapRecord in Vec<CapRecord>]

  → Revocation:
    → collar_revoke_grant()  [sets CollarGrant state=Revoked]
    → collar_revoke_sexfiles_caps()  [invalidates persisted cap records]
      → pdx_call(SLOT_LINEN, OP_LINEN_REVOKE_CAPS)
        → Linen handle_linen_revoke_caps()
          → pdx_storage_sync(OP_RAMFS_REVOKE_CAPS)
            → RamFS.revoke_caps_by_object_id()  [bumps cap_generation + invalidates records]
```

## Enforcement

Two enforcement points:

1. **Collar-level** (silk-shell): `collar_check_operation()` checks in-memory
   `COLLAR_GRANTS` for Active grants with matching subject/operation.

2. **SexFiles-level** (persisted): RamFS `CapRecord` entries validated at file
   access time via `check_access()`. Collar grants create persisted CapRecords
   that survive reboot reconstruction.

Dangerous operations (`AccessDisplay`, `AccessShellPolicy`) are always denied.

## New Opcodes

| Layer | Opcode | Value | Purpose |
|-------|--------|-------|---------|
| SexFiles | OP_RAMFS_GRANT_CAP | 0x39 | Grant capability by object_id + subject_pd |
| SexFiles | OP_RAMFS_REVOKE_CAPS | 0x3A | Revoke all caps for an object |
| Linen | OP_LINEN_GRANT_CAP | 0x46 | Bridge: forward grant to SexFiles |
| Linen | OP_LINEN_REVOKE_CAPS | 0x47 | Bridge: forward revoke to SexFiles |

All opcodes are local constants per server; no sex-pdx ABI changes.

## Files Changed

| File | Changes |
|---|---|
| `servers/sexfiles/src/messages.rs` | +OP_RAMFS_GRANT_CAP (0x39), +OP_RAMFS_REVOKE_CAPS (0x3A) |
| `servers/sexfiles/src/backends/ramfs.rs` | +grant_cap_by_object_id(), +revoke_caps_by_object_id() |
| `servers/sexfiles/src/vfs.rs` | +dispatch for 0x39, 0x3A |
| `servers/linen/src/main.rs` | +OP_LINEN_GRANT_CAP (0x46), +OP_LINEN_REVOKE_CAPS (0x47), +handlers |
| `servers/silk-shell/src/main.rs` | +OP_LINEN_GRANT_CAP, +OP_LINEN_REVOKE_CAPS, +persist/revoke helpers, +proof gate |
| `apps/spindle/src/main.rs` | +EventRing fix (pre-existing breakage in input proof) |

## Proof Markers (SEXOS_COLLAR_APP_CAP_GRANT_PROOF=1)

All 6 stages run in synthetic boot loop:

| Stage | Marker | Meaning |
|-------|--------|---------|
| 0 | `[collar.appcap.proof.review]` | Manifest with SEXFILES cap → CollarReview.allowed=true |
| 1 | `[collar.appcap.proof.grant]` | Auto-grant creates Active CollarGrant |
| 2 | `[collar.appcap.proof.allow]` | collar_check_operation(AccessSexFiles) → Allow |
| 3 | `[collar.appcap.proof.revoke]` | collar_revoke_grant → state=Revoked |
| 4 | `[collar.appcap.proof.revoked_deny]` | Post-revoke check → Deny |
| 5 | `[collar.appcap.proof.dangerous_deny]` | AccessDisplay + AccessShellPolicy → Deny |

## Build/Runtime Result

```
$ cargo build -p sexfiles --target x86_64-unknown-none   → OK (0 warnings)
$ cargo build -p linen --target x86_64-unknown-none      → OK (0 warnings)
$ cargo build -p silk-shell --target x86_64-unknown-none → OK (pre-existing warnings only)

$ ./scripts/entrypoint_build.sh
ISO image produced: 1708 sectors
[SEXOS ENTRYPOINT] success
```

## Remaining Authority Gaps

1. **No real Collar PD**: Collar decisions are shell-local. No separate Collar PD with PDX-level authority enforcement. Kernel capability system (MPK/PKU/PKEY) remains the real boundary.

2. **Shell-local caller identity**: Collar derives caller identity from `FOCUSED_SURFACE_ID`. In a multi-PD Collar, caller identity would come from PDX `caller_pd`.

3. **No persistent media**: All grant/cap records are in-memory (both Collar grants and RamFS CapRecords). A QEMU power cycle resets everything. True persistence requires the real block device route (blocked per SEXFILES_REAL_BLOCK_BACKEND_V1.md).

4. **No UI**: Collar has no user-facing capability management UI. The proof is internal markers only. A user cannot inspect, grant, or revoke capabilities interactively.

5. **No time/context scoping**: Grants are simple `(subject_id, operation_mask)`. No expiration, no rate limiting, no per-operation context validation.

6. **Linen bridge dependency**: Collar persistence depends on Linen having a pre-created session object with `sexfiles_object_id > 0`. Without Linen objects in the session table, the grant persistence path has no global object_id to anchor to.

## STOP Conditions

- No kernel capability rewrite needed
- No sex-pdx ABI edit (opcodes are local server constants)
- No app-controlled security policy (Collar decides, not the app)
- No broad Collar redesign (additive, not restructured)
