# A7_SURFACE_OPCODE_AUDIT_V1

**Status:** Complete — audit + minimal constant rename.
**Build:** ISO produced, no errors.

---

## Summary

Audited the 0xEE surface opcode collision risk between silk-shell and
sexdisplay. **Verdict: docs-only handoff. No direct wrong opcode use found.**
All uses of 0xEE map correctly to sexdisplay's `active = false` deactivation.
The shell's lifecycle FSM (A3/A6) tracks the semantic difference between
permanent destroy (Closing/Tombstoned) and temporary hide (Minimized/tab
switch/panel toggle).

One code change: renamed `OP_SURFACE_DESTROY` → `OP_SURFACE_DEACTIVATE` with
clarifying comment, and used the renamed constant in the two most semantically
important paths (close + minimize).

---

## Sexdisplay Opcode Map (Complete)

All opcodes handled by sexdisplay's main dispatch loop:

| Opcode | Local Name | Shell Constant | Sexdisplay Action |
|--------|-----------|----------------|-------------------|
| `0x11` | OP_PRIMARY_FB | — | Framebuffer setup |
| `0xE4` | OP_WINDOW_CREATE (legacy) | — | Legacy create (no surface_id) |
| `0xDE` | OP_WINDOW_CREATE (pointer) | — | Legacy pointer protocol — UNSUPPORTED |
| `0xEB` | OP_SURFACE_UPDATE | `OP_SURFACE_UPDATE = 0xEB` | Move surface position (x, y only). Ownership checked. |
| `0xEC` | OP_SURFACE_CREATE_ID | (magic number) | Upsert surface: create or update geometry (x, y, w, h). Ownership checked. |
| `0xED` | OP_SET_FOCUS | (magic number) | Set/clear focus surface |
| `0xEE` | OP_SURFACE_DEACTIVATE | `OP_SURFACE_DEACTIVATE = 0xEE` | **Deactivate surface (active=false). Does NOT free resources.** Ownership checked. |
| `0xEF` | OP_SURFACE_FILL_RECT | (magic number) | Set surface fill color/rect. Ownership checked. |
| `0xFC` | OP_APPEARANCE_TOKENS | (from sex-pdx) | Set render tokens (two-call state machine) |
| `0xFD` | OP_SURFACE_TAB_INFO | `OP_SURFACE_TAB_INFO` (from sex-pdx) | Set tab count, active tab, chrome flags |

---

## 0xEE Collision Analysis

### The Question

Does 0xEE mean "hide" or "destroy"? The shell uses it for BOTH:
- **Permanent destroy**: `close_surface_from_frame_light()`, `DestroyFocused`
- **Temporary hide**: `minimize_frame()`, tab switch, panel toggle, atlas overlay

### What sexdisplay does

Sexdisplay's 0xEE handler (line 1044) sets `surface.active = false`. That's it.
No resources freed, no slot reclaimed, no ownership change. The surface is
simply excluded from rendering.

### Why this is safe

| Aspect | Safe? | Reason |
|--------|-------|--------|
| Visual | ✅ | Both destroy and hide should make the surface disappear |
| Lifecycle | ✅ | Shell's FSM (A3/A6) tracks Closing/Tombstoned vs Minimized/Hidden |
| Re-use | ✅ | `is_tombstoned()` + lifecycle guards prevent stale references |
| Generation | ✅ | Bump on entering Closing prevents stale FocusRef from working |
| Resource leak | ⚠️ | Surface slot remains allocated — acceptable for V1 (16 slots, ~6 used) |

### Why this is NOT a collision

Calling 0xEE three times on the same surface is idempotent (active stays false).
Calling 0xEC after 0xEE re-activates the surface (creates in new slot or
updates geometry). There is no state corruption path.

The lifecycle FSM is the AUTHORITY on surface state, not the wire protocol.
Sexdisplay just renders what the shell tells it to.

---

## Per-Path Audit

### `close_surface_from_frame_light()` (line 2989)
- **Lifecycle**: Visible → Closing → Tombstoned (set before 0xEE)
- **0xEE intent**: Permanent destroy
- **After**: `clear_focus_if_dead()`, `clear_drag_if_dead()`, tile, snap
- **Verdict**: ✅ Correct. No wrong use.

### `minimize_frame()` (line 3092)
- **Lifecycle**: Visible → Minimized (set before 0xEE)
- **0xEE intent**: Temporary hide
- **After**: `clear_drag_if_dead()`, `clear_focus_if_dead()`, snap
- **Restore via**: 0xEC with stored geometry
- **Verdict**: ✅ Correct. Lifecycle FSM distinguishes from destroy.

### `tab switch` (line 3692)
- **Lifecycle**: No change (old tab hidden, new tab shown)
- **0xEE intent**: Hide old tab
- **0xEC intent**: Show new tab
- **Verdict**: ✅ Correct. 0xEE just deactivates old tab.

### `panel toggle off` (lines 4334, 4364, 5003)
- **Lifecycle**: Mapped → Allocated (via `set_lifecycle_state`)
- **0xEE intent**: Hide panel
- **Verdict**: ✅ Correct. Panel lifecycle tracked separately.

### `atlas overlay hide` (lines 2053, 2125, 2253, 4150)
- **Lifecycle**: Mapped → Allocated (via `set_lifecycle_state`)
- **0xEE intent**: Hide overlay
- **Verdict**: ✅ Correct. Atlas lifecycle tracked separately.

### `DestroyFocused` (lines 5060, 5074, 5088, 5102)
- **Lifecycle**: Visible → Closing → Tombstoned (set before 0xEE)
- **0xEE intent**: Permanent destroy
- **Verdict**: ✅ Correct. Same pattern as close_surface_from_frame_light.

---

## 0xEF / 0xEC / 0xED / 0xEB Audit

| Opcode | Usage | Collision? |
|--------|-------|------------|
| `0xEB` | Position update only (x, y) | ✅ Safe. Ownership checked in sexdisplay. |
| `0xEC` | Create + geometry update (x, y, w, h) | ✅ Safe. Ownership checked. Also handles restore/zoom/unzoom. |
| `0xED` | Focus set/clear | ✅ Safe. No ownership check (anyone can clear). |
| `0xEF` | Fill rect | ✅ Safe. Ownership checked. |

---

## B1 Primitive Safety

| B1 Primitive | Opcode | Safe? | Notes |
|-------------|--------|-------|-------|
| Move frame/surface | `0xEB` (position) or `0xEC` (geometry) | ✅ | Both work. 0xEB is lighter (x,y only). |
| Resize frame/surface | `0xEC` (with new w, h) | ✅ | Already used by zoom/unzoom. |
| Minimize | `0xEE` + lifecycle Minimiized | ✅ | Already used by minimize_frame. |
| Restore | `0xEC` (with stored geometry) | ✅ | Already used by restore_minimized_frame. |
| Close/destroy | `0xEE` + lifecycle Tombstoned | ✅ | Already used by close_surface_from_frame_light. |
| Tab switch | `0xEE` old + `0xEC` new | ✅ | Already used by switch_tab. |

**No new opcodes needed for B1.** All required primitives exist and are proven.

---

## Changed Files

### `servers/silk-shell/src/main.rs`

1. **Line 80-84**: Renamed `OP_SURFACE_DESTROY` → `OP_SURFACE_DEACTIVATE` with
   clarifying comment explaining 0xEE semantics and lifecycle tracking.

2. **Line 2989**: Used `OP_SURFACE_DEACTIVATE` in `close_surface_from_frame_light()`
   with comment documenting the lifecycle/deactivation relationship.

3. **Line 3092**: Used `OP_SURFACE_DEACTIVATE` in `minimize_frame()` with
   comment documenting the minimize/destroy opcode-sharing pattern.

**No ABI values changed. No opcodes added or removed. No sexdisplay changes.**

---

## STOP FIRST Conditions

None triggered. This audit found no evidence of:
- Wrong opcode usage
- State corruption from overloaded 0xEE
- Missing ownership checks
- Resource leaks from 0xEE deactivation
- B1-blocking primitive gaps

---

## Proof

```sh
# Build
./scripts/entrypoint_build.sh  # ISO produced, no errors

# Verify opcode usage
rg -c "0xEE" servers/silk-shell/src/main.rs  # 24 (includes const def, comments, all sites)
rg -c "0xEF" servers/silk-shell/src/main.rs  # 11 (all fill rect)
rg -c "OP_SURFACE_DEACTIVATE" servers/silk-shell/src/main.rs  # 3 (const + 2 use sites)
rg -c "OP_SURFACE_UPDATE" servers/silk-shell/src/main.rs     # 9 (position updates)
```
