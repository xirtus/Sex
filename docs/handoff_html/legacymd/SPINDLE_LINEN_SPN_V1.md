# SPINDLE_LINEN_SPN_V1

**Date:** 2026-05-06
**Status:** Local session active — Linen .spn bridge pending kernel capability grant
**Previous:** SPINDLE_BELL_BRIDGE_V1
**Next:** SPINDLE_DISPLAY_SURFACE_V1 (Phase 7)

---

## Summary

Linen .spn session object status update:
- SexObject kind `SpindleSession = 5` with `.spn` extension canon (sex-object-model)
- `session` command shows local summary with honest pending status
- Serial marker: `[spindle.linen.spn.pending] reason=no_linen_cap kind=SpindleSession ext=.spn`
- SLOT_LINEN=13 exists in sex-pdx

---

## Unblock Condition

```rust
// kernel/src/init.rs — 1 line
pd.grant_capability(sex_pdx::SLOT_LINEN, CapabilityData::Domain(spindle_id));
```

After this: `pdx_call(SLOT_LINEN, OP_LINEN_CREATE_OBJECT, ...)` to publish .spn file.

---

## SexObject Chain (Complete)

| Layer | Definition |
|-------|-----------|
| SexObjectKind | `SpindleSession = 5` in `crates/sex-object-model/src/lib.rs` |
| Extension canon | `.spn` — `SexObjectKind::SpindleSession => ".spn"` |
| SexfilesObjectEntry | `kind: u16` (5 = SpindleSession) |
| Linen display | `.spn` file visible in Linen browser (when cap granted) |

---

## Commands

```
sex> session
Spindle session summary:
  session id:  1 (local)
  commands:    Spindle native command console
  history:     pending (SexFiles bridge)
  events:      pending (Bell bridge)
Linen bridge pending (capability grant pending).
```

---

## Three Pending Capability Grants

| Cap | Slot | Unblocks |
|-----|------|----------|
| `SLOT_STORAGE` | 1 | SexFiles history persistence |
| `SLOT_BELL` | 12 | Bell event bridge |
| `SLOT_LINEN` | 13 | Linen .spn session object |

All three are **1-line kernel init.rs edits**:
```rust
pd.grant_capability(sex_pdx::SLOT_STORAGE, CapabilityData::Domain(spindle_id));
pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(spindle_id));
pd.grant_capability(sex_pdx::SLOT_LINEN, CapabilityData::Domain(spindle_id));
```

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| entrypoint_build.sh | PASS |
| master_runtime_gate | **GREEN_MASTER** (6/6) |
| Faults | **0** |

### Serial Log

```
[spindle.sexfiles.persist.pending] reason=no_storage_cap
[spindle.bell.pending] reason=no_bell_cap
[spindle.linen.spn.pending] reason=no_linen_cap kind=SpindleSession ext=.spn
```

---

## Next Prompt

```
SPINDLE_DISPLAY_SURFACE_V1
```

Phase 7: Display/surface integration through silk-shell.
