# SHELL_FOCUS_CONTRACT_V1

**Date:** 2026-05-03
**Status:** MERGED

## Symptom

`is_focusable_surface()` exists as dead code — defined at line 375 but never called. All focus write sites in `silk-shell` directly assign `FOCUSED_SURFACE_ID` and call `pdx_call(0xED)` without any formal guard against nonfocusable or dead surfaces.

While the click-focus paths implicitly exclude panels/cursor via z-order iteration (only contains app IDs + linen), keyboard shortcuts (FocusToggle, Focus100-Focus103, Focus200) do not consistently check alive status. FocusToggle in particular cycles IDs blindly without checking `surface_is_alive()` — if a surface is destroyed, the cycle can set focus to a dead surface.

## Root Cause

No centralized focus guard function exists. Every focus write site independently handles (or fails to handle) validity checks:

- `is_focusable_surface()` — defined at line 375 but **never called** anywhere
- `surface_is_alive()` — called in click-focus paths but NOT in FocusToggle keyboard shortcut
- `point_in_surface()` — explicitly rejects panels/cursor with `[shell.surface.nonfocusable.reject]` but only for hit-test, not for keyboard focus shortcuts

## Fix

### New `try_set_focus()` guard function

Added at line 382, replacing all direct `FOCUSED_SURFACE_ID = X; pdx_call(0xED, X)` with a single guarded path:

```rust
unsafe fn try_set_focus(sid: u64) -> bool {
    if sid == 0 {
        FOCUSED_SURFACE_ID = 0;
        pdx_call(SLOT_DISPLAY, 0xED, 0, 0, 0);
        return true;
    }
    if !is_focusable_surface(sid) {
        serial_println!("[shell.focus.reject.nonfocusable] id={}", sid);
        return false;
    }
    if !surface_is_alive(sid) {
        serial_println!("[shell.focus.reject.dead] id={}", sid);
        return false;
    }
    FOCUSED_SURFACE_ID = sid;
    pdx_call(SLOT_DISPLAY, 0xED, sid, 0, 0);
    true
}
```

### All focus write sites replaced

Every site that previously wrote `FOCUSED_SURFACE_ID` and called `pdx_call(0xED)` now calls `try_set_focus()`:

| Site | Lines | Behavior preserved? |
|------|-------|---------------------|
| `clear_focus_if_dead()` fallback | ~337 | ✅ — z-order already filter focusable+alive |
| `clear_focus_if_dead()` clear none | ~349 | ✅ — sid=0 special case |
| USB_MOUSE_REPORT click-focus | ~671 | ✅ — hit_id already verified alive in z-order |
| EV_BTN click-focus | ~1462 | ✅ — same |
| FocusToggle keyboard | ~756 | ✅ — now checks alive (was missing) |
| DestroyFocused auto-focus | ~800 | ✅ — already checked `_ALIVE` booleans |
| Focus100-Focus103 shortcuts | ~822 | ✅ — already checked `_ALIVE` |
| Focus200 shortcut | ~852 | ✅ — LINEN always alive |
| RecreateFocused | ~899 | ✅ — surface recreated before focus |
| ResetAll | ~927 | ✅ — surfaces recreated before focus |

The only behavior change: **FocusToggle now refuses to focus dead surfaces** (previously it cycled blindly). All other paths already had equivalent checks — `try_set_focus()` formalizes them into one consistent guard.

## Changed Invariants

1. `try_set_focus()` is the sole gate for focus assignment. Direct `FOCUSED_SURFACE_ID` writes outside this function are forbidden.
2. All focus targets must pass `is_focusable_surface()` (panels/cursor rejected) AND `surface_is_alive()` (dead surfaces rejected).
3. Clearing focus (sid=0) bypasses both checks — always allowed.
4. `[shell.focus.reject.nonfocusable]` fires when a panel or cursor ID is attempted as focus (unbudgeted — reject marker).
5. `[shell.focus.reject.dead]` fires when a destroyed surface is attempted as focus (unbudgeted — reject marker).
6. Click-focus behavior unchanged (same surfaces in z-order, same alive checks).

## Marker List

| Marker | Type | Budget | When |
|--------|------|--------|------|
| `[shell.focus.reject.nonfocusable]` | reject | unbudgeted | Nonfocusable surface (panel/cursor) targeted for focus |
| `[shell.focus.reject.dead]` | reject | unbudgeted | Dead/destroyed surface targeted for focus |

## Verification

```bash
./scripts/entrypoint_build.sh

SEXUSB_XHCI_TRACE=0 timeout 12 ./dev.sh run-nographic \
  2>/tmp/shell-focus-contract.trace | tee /tmp/shell-focus-contract.log

# Verify no invalid focus attempts
grep -cE 'shell.focus.reject' /tmp/shell-focus-contract.log   # = 0

# Verify click-focus still works
grep -c 'shell.click_focus' /tmp/shell-focus-contract.log     # > 0

# Verify drag still works
grep -c 'shell.drag' /tmp/shell-focus-contract.log            # > 0

# Verify no faults/panics
grep -cE 'fault|panic' /tmp/shell-focus-contract.log          # = 0

# Verify all proof markers preserved
grep -c 'shell.silkbar.click' /tmp/shell-focus-contract.log   # ≥ 7
grep -c 'silk.render_proof.top_strip.ok' /tmp/shell-focus-contract.log  # = 1
```

## Verified Results (2026-05-03)

```
shell.focus.reject.*:             0 (no invalid focus attempts)
shell.click_focus:                 2 (click-focus working)
shell.drag:                        4 (drag working)
fault/panic:                       0
shell.surface.(nonfocusable|unknown).reject: 0
panel opens/closes:                >4 (launcher, clock, status, bell)
silk.render_proof.top_strip.ok:    1
```

## STOP FIRST Conditions

1. Changes to kernel/sex-pdx/sexdisplay/sexinput
2. Adding new focus write sites outside `try_set_focus()`
3. Removing or bypassing `is_focusable_surface()` check
4. Adding nonfocusable surfaces (panels/cursor) to focus z-order
5. Broad refactor of the focus model
6. New allocator or tombstone registry
