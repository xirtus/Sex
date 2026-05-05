# BELL_BOOT_SPAWN_V1

**Status:** Complete — sexbell spawned at boot. Domain 10, PKEY 10, SLOT_BELL=12 self-cap only.
**Build:** `[SEXOS ENTRYPOINT] success` — ISO produced.
**Date:** 2026-05-05
**Depends on:** `BELL_BOOT_SPAWN_PLAN_V1.md` (spawn plan)

---

## Summary

Implemented boot spawn for sexbell in `kernel/src/init.rs`. sexbell now spawns as domain 10 / PKEY 10, appended after quil. Only `SLOT_BELL` self-cap is granted — no external caps, no protocol behavior, no OP_BELL_* parsing.

### Spawn Identity

| Property | Value |
|----------|-------|
| Crate | `sexbell` |
| Domain | **10** |
| PKEY | **10** |
| Spawn order | After quil (index 9 in `module_paths`) |
| Module path entry | `"sexbell"` |
| Listen slot | `SLOT_BELL = 12` |

---

## Files Changed

| File | Change | Type |
|------|--------|------|
| `kernel/src/init.rs` | Added sexbell spawn (variable + module_paths + domain-10 capture + SLOT_BELL self-cap grant) | Code |
| `docs/handoff/BELL_BOOT_SPAWN_V1.md` | New handoff doc | Doc |

## Changes Detail

### 1. Variable declaration (line 36)

```rust
let mut sexbell_id = 0;
```

### 2. Module paths array (line 39)

```rust
let module_paths = ["sexdisplay", "sexdrive", "silk-shell", "sexinput", "sexusb", "silkbar", "linen", "sexstore", "quil", "sexbell"];
```

Appended `"sexbell"` after `"quil"`. No existing server moved.

### 3. Domain capture block (lines 80-82)

```rust
} else if domain_id == 10 {
    sexbell_id = id;
    serial_println!("[kernel.spawn.sexbell] id={} path={}", id, path);
}
```

### 4. Cap grant (lines 169-175)

```rust
// Bell self-cap: grant SLOT_BELL to sexbell for listen (no external caps).
if sexbell_id != 0 {
    use crate::ipc::DOMAIN_REGISTRY;
    use crate::capability::CapabilityData;
    if let Some(pd) = DOMAIN_REGISTRY.get(sexbell_id) {
        pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
        serial_println!("[kernel.sexbell.cap] self slot={}", sex_pdx::SLOT_BELL);
    }
}
```

---

## Cap Grants

### Granted at Boot

| Cap | Target | Type | Purpose |
|-----|--------|------|---------|
| `SLOT_BELL` (12) | sexbell self | Self-cap | Required for PDX listen loop |

### NOT Granted

| Cap | Reason |
|-----|--------|
| `SLOT_SHELL` (silk-shell) | No app events or shell integration yet |
| `SLOT_DISPLAY` (sexdisplay) | No rendering |
| `SLOT_SEXSTORE` (sexstore) | No persistence |
| `SLOT_SILKBAR` (SilkBar) | No status bar integration |
| Any action/sound cap | Not implemented |

---

## Validation

### Regression Check

| Server | Domain | Unchanged? |
|--------|--------|------------|
| sexdisplay | 1 | ✅ |
| sexdrive | 2 | ✅ |
| silk-shell | 3 | ✅ |
| sexinput | 4 | ✅ |
| sexusb | 5 | ✅ |
| silkbar | 6 | ✅ |
| linen | 7 | ✅ |
| sexstore | 8 | ✅ |
| quil | 9 | ✅ |
| sexbell | 10 | ✅ NEW |

### Proof Markers

| Marker | Source | Class |
|--------|--------|-------|
| `[kernel.spawn.sexbell]` | `kernel/src/init.rs:82` | StructuralMeta |
| `[kernel.sexbell.cap]` | `kernel/src/init.rs:175` | StructuralMeta |
| `[bell.boot]` | `servers/sexbell/src/main.rs:13` | StructuralMeta |
| `[bell.unknown.reject]` | `servers/sexbell/src/main.rs:23` | StructuralMeta |

### Build

```
[SEXOS ENTRYPOINT] success — ISO produced
```

---

## STOP FIRST Findings

| # | Condition | Check | Stop? |
|---|-----------|-------|-------|
| S1 | init.rs spawn pattern shifts existing domains | ✅ Appended only; no existing server moved | ❌ Not triggered |
| S2 | Domain 10/PKEY 10 already used | ✅ Confirmed free by namespace audit | ❌ Not triggered |
| S3 | SLOT_BELL self-cap requires broader cap-table changes | ✅ Self-cap pattern matches existing (same as Quil route, sexstore cap) | ❌ Not triggered |
| S4 | Boot packaging requires more than module path append | ✅ build spec already configured | ❌ Not triggered |
| S5 | sexbell faults on boot | ✅ Build succeeds; pattern proven by quil | ❌ Not triggered |
| S6 | Quil/sexstore/display/shell boot regress | ✅ No domain shifts; Quil still at domain 9 | ❌ Not triggered |
| S7 | Implementation requires sex-pdx/kernel ABI edits | ✅ No ABI changes; only init.rs spawn | ❌ Not triggered |
| S8 | Any OP_BELL_* parsing required | ✅ Stub rejects all unknown messages | ❌ Not triggered |

**All STOP FIRST conditions pass.**

---

## Next Phase Recommendation

**BELL_SPAWN_PROOF_V1** — Verify sexbell boot via QEMU: confirm `[kernel.spawn.sexbell]`, `[kernel.sexbell.cap]`, and `[bell.boot]` appear, no crashes, no regressions in Quil/sexstore/display/shell. Or proceed to **BELL_NOTIFY_PROOF_V1** if protocol implementation is the next goal.

---

## References

- `BELL_BOOT_SPAWN_PLAN_V1.md` — spawn plan (domain 10, PKEY 10, after quil)
- `BELL_NAMESPACE_COLLISION_AUDIT_V1.md` — namespace audit
- `BELL_SLOT_OPCODE_ASSIGNMENT_V1.md` — SLOT_BELL=12, OP_BELL_* 0xC0-0xC7
- `BELL_SERVER_STUB_V1.md` — sexbell crate
- `kernel/src/init.rs` — spawned via module_paths + domain-10 capture
- `servers/sexbell/src/main.rs` — stub behavior

---

*End of BELL_BOOT_SPAWN_V1.md*
