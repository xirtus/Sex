# K9: Scope Linen Open Trigger to Focus

**Status:** Handoff (code + docs)
**Commit:** *(to be committed)*
**Purpose:** Remove the last remaining global debug trigger by scoping PrintScreen
(0x59) open action to Linen focus, matching J/K selection gating.

## 1. Changes

### 1.1 Gate Addition (`servers/silk-shell/src/main.rs`)

**Before (K4):**
```rust
SurfaceAction::OpenObjectInQuil => {
    let obj_id = linen_selected_object_id();
    if obj_id == 0 {
        serial_println!("[linen.quil.open.reject.no_selection]");
    } else if open_linen_object_in_quil(obj_id) {
        mutated = true;
        serial_println!("[shell.action.open_object_in_quil] object_id={}", obj_id);
    }
}
```

**After (K9):**
```rust
SurfaceAction::OpenObjectInQuil => {
    if FOCUSED_SURFACE_ID == SURFACE_ID_LINEN {
        let obj_id = linen_selected_object_id();
        if obj_id == 0 {
            serial_println!("[linen.quil.open.reject.no_selection]");
        } else if open_linen_object_in_quil(obj_id) {
            mutated = true;
            serial_println!("[shell.action.open_object_in_quil] object_id={}", obj_id);
        }
    } else {
        serial_println!("[linen.quil.open.reject] reason=not_focused");
    }
}
```

### 1.2 Gating Summary

| Trigger | Gate | Before K9 | After K9 |
|---------|------|-----------|----------|
| J (0x24) — select next | `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN` | ✅ Gated | ✅ Gated |
| K (0x25) — select prev | `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN` | ✅ Gated | ✅ Gated |
| PrintScreen (0x59) — open | `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN` | ❌ Global | ✅ Gated |

## 2. Proof Markers

| Marker | When |
|--------|------|
| `[linen.quil.open.reject] reason=not_focused` | PrintScreen pressed while Linen not focused |
| `[linen.quil.open.reject.no_selection]` | PrintScreen pressed, Linen focused, no objects |
| All existing `[linen.quil.open.*]` markers | Normal path when Linen focused |

All three triggers now produce `[reason=not_focused]` reject when gating fires. Consistent proof.

## 3. K8 Risk Resolved

K8 §6 listed "PrintScreen global debug trigger" as LOW remaining risk. **Resolved.** PrintScreen
is now scoped to Linen focus only. All three Linen keyboard triggers (J, K, PrintScreen) use
identical gating.

## 4. Files Changed

- `servers/silk-shell/src/main.rs` — added focus guard around OpenObjectInQuil handler (~5 lines)
- `docs/handoff/K9_SCOPE_LINEN_OPEN_TRIGGER_V1.md` — this document

## 5. Verification

- **Build:** `./scripts/entrypoint_build.sh` passes, ISO produced
- **No changes:** kernel/ABI/sex-pdx, sexdisplay, lifecycle, storage, editor, J4-J7 internals
- **Consistent gating:** All 3 Linen keyboard triggers use identical `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN` check
- **Reject marker:** `[linen.quil.open.reject] reason=not_focused` emitted when not focused
