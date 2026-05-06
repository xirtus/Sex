# COLLAR_CAPABILITY_REVIEW_MODEL_V1

**Status:** Implemented (internal model/proof, gated by SEXOS_COLLAR_REVIEW_PROOF=1)
**Date:** 2026-05-06
**Files changed:** 1 (+308 / -5 lines)
**Build:** PASS (1685 sectors)
**Runtime Gate:** GREEN_MASTER

---

## Route Chosen

Made Collar real as the user-facing capability review/trust model by:
1. Adding system capability operations (AccessBell, AccessSexFiles) to CollarOperation
2. Renaming `collar_check_operation_stub` to `collar_check_operation` (was already doing real grant lookups)
3. Adding `collar_auto_grant_from_manifest()` — auto-creates Collar grants from AppManifest capability bits
4. Adding `collar_review_manifest()` — reviews a manifest against policy without creating grants
5. Adding 5-stage boot proof validating the review model

### CollarOperation Variants (V3)

| Variant | Code | Policy |
|---------|------|--------|
| OpenObject | 0 | Grant table lookup |
| RenameObject | 1 | NeedsGrantLater |
| ArchiveObject | 2 | NeedsGrantLater |
| SaveBuffer | 3 | BlockedStopFirst |
| BuildTarget | 4 | BlockedStopFirst |
| RunTarget | 5 | BlockedStopFirst |
| LinkObjectToBuffer | 6 | Grant table lookup |
| **AccessBell** | **7** | **Grant table lookup (new)** |
| **AccessSexFiles** | **8** | **Grant table lookup (new)** |
| **AccessDisplay** | **9** | **Always deny (new)** |
| **AccessShellPolicy** | **10** | **Always deny (new)** |

### Capability Review Rules

| Request | Collar Decision | Reason |
|---------|----------------|--------|
| BELL bit | Grant if subject has active grant | Auto-granted from manifest |
| SEXFILES bit | Grant if subject has active grant | Auto-granted from manifest |
| Unknown bit (>0x03) | Deny | Not in KNOWN mask |
| Raw framebuffer/display | Deny | Not representable in manifest + explicit CollarOperation deny |
| Shell policy ownership | Deny | Not representable in manifest + explicit CollarOperation deny |

### Auto-Grant Flow

```
App surface request (OP_APP_SURFACE_REQ, 0xFA)
  -> AppManifest::unpack() validates manifest
  -> handle_app_surface_req() accepts
  -> collar_auto_grant_from_manifest() creates Active grants
      -> BELL cap -> CollarGrant { operation_mask: AccessBell }
      -> SEXFILES cap -> CollarGrant { operation_mask: AccessSexFiles }
  -> Frame created, surface registered
```

## Proof Markers (gated by SEXOS_COLLAR_REVIEW_PROOF=1)

All 5 stages run at boot in the main listen loop:

```
[collar.review.proof] stage=0
[collar.review] sid=400 app_id=1 requested=0x2 granted=0x2 denied=0x0 allowed=true
[collar.review.proof.1] sexfiles_cap_allowed=true

[collar.review.proof] stage=1
[collar.review] sid=401 app_id=2 requested=0x3 granted=0x3 denied=0x0 allowed=true
[collar.review.proof.2] bell_sexfiles_allowed=true

[collar.review.proof] stage=2
[collar.review.proof.3] unknown_cap_rejected=true
[collar.review.proof.3b] validate_rejects_unknown=true

[collar.review.proof] stage=3
[collar.review] sid=403 app_id=0 requested=0x0 granted=0x0 denied=0x0 allowed=true
[collar.review.proof.4] no_caps_allowed=true

[collar.review.proof] stage=4
[collar.policy.check] op=9 object_id=0 buffer_id=0 caller_sid=201
[collar.gate.reject] reason=always_deny op=9
[collar.policy.check] op=10 object_id=0 buffer_id=0 caller_sid=201
[collar.gate.reject] reason=always_deny op=10
[collar.review.proof.5] display_policy_always_denied=true
```

## Files Changed

```
servers/silk-shell/src/main.rs  +308 / -5  (Collar review model + proof)
```

No kernel edits. No sex-pdx ABI changes. No new crates.

## Build / Runtime

- Build: `./scripts/entrypoint_build.sh` -- PASS (ISO: 1685 sectors)
- Runtime gate: `SEXOS_COLLAR_REVIEW_PROOF=1 ./scripts/master_runtime_gate.sh --probe 35` -- GREEN_MASTER
  - All 6 PDs spawn and run
  - Clock liveness: 12 ticks
  - No faults/panics
  - All 5 Collar review proof stages pass

## Remaining Authority Risks

1. **No real Collar PD**: All Collar decisions are shell-local. No separate Collar PD with PDX-level authority enforcement. Kernel capability system remains the real boundary.
2. **Shell-local caller identity**: Collar derives caller identity from FOCUSED_SURFACE_ID. In a multi-PD Collar, caller identity would come from PDX caller_pd.
3. **No persistent secrets/storage**: Collar grants are in-memory only. A reboot resets all grants.
4. **No UI**: Collar has no user-facing capability review UI. The proof is internal markers only.
5. **No kernel enforcement**: Collar review is advisory. Real enforcement is via kernel capability system (MPK/PKU/PKEY).

## STOP FIRST Conditions Met

- No kernel capability system change
- No sex-pdx ABI change
- No persistent secrets/storage
- No app-controlled security policy
- No renderer policy
