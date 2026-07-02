# SURFACE_ACTION_BINDINGS_V1

**Status:** Active  
**Purpose:** Add keyboard bindings for Linen and Quil shell-managed surfaces via existing SurfaceAction dispatch.  
**Scope:** `servers/silk-shell/src/main.rs` only. No kernel/ABI/sexdisplay changes.  
**Prerequisites:** `LINEN_SURFACE_CONTROL_V1` (f9100b5), `QUIL_SURFACE_STUB_V1` (98cadaf)

---

## What Was Done

Linen and Quil are now user-openable via keyboard shortcuts through the existing scancode-to-SurfaceAction dispatch path. No QEMU input, no server binaries, no editor logic — purely shell action routing.

### Changes

| Location | Change |
|----------|--------|
| `SurfaceAction` enum (line 476-477) | Added `ToggleLinen`, `ToggleQuil` variants |
| `scancode_to_action()` (line 572-573) | `0x42 (F8) → ToggleLinen`, `0x43 (F9) → ToggleQuil` |
| Dispatch handler (line 4063-4068) | `ToggleLinen` → calls `toggle_linen()` |
| Dispatch handler (line 4070-4075) | `ToggleQuil` → calls `toggle_quil()` |

### Behavior

| Shortcut | Action | Effect |
|----------|--------|--------|
| F8 | `ToggleLinen` | Toggle Linen visibility: minimize if visible, open if not. Sets `mutated = true` on state change. |
| F9 | `ToggleQuil` | Toggle Quil visibility: minimize if visible, open if not. Sets `mutated = true` on state change. |

Both actions:
- Use existing `toggle_linen()` / `toggle_quil()` helpers (lazy frame creation, scene-aware)
- Call `snap_capture_layout()` through the helper (layout mutation tracking)
- Fire proof markers `[shell.action.linen]` and `[shell.action.quil]` on successful toggle
- Preserve all existing guards (active scene check, frame_accepts_input, focus validity)
- No new input framework — pure synthetic key dispatch through existing scancode path

### Existing Action Path Summary

The silk-shell dispatch for keyboard input works as follows:

1. `OP_HID_EVENT` (0x202) delivers scancode from HID server
2. Special scancodes (0x41=F7 scene settings) handled before normal dispatch
3. Normal dispatch: `scancode_to_action(scancode)` → `Option<SurfaceAction>`
4. `match action { ... }` dispatches to handlers
5. Handlers call shell helpers, set `mutated = true`, log markers
6. `snap_capture_layout()` called after layout mutations

Actions added by this phase (F8, F9) follow this exact path with zero new infrastructure.

---

## Build Result

```
RUSTFLAGS="-C relocation-model=pic -C link-arg=-pie" cargo build \
    -Z build-std=core,compiler_builtins,alloc \
    -Z build-std-features=compiler-builtins-mem \
    --manifest-path servers/silk-shell/Cargo.toml \
    --target /home/xirtus_arch/x86_64-sex.json \
    --release

Finished release profile [optimized] in 1.00s
Warnings: 201 bin + 1 lib (all pre-existing)
Errors: 0
```

## Files Changed

- `servers/silk-shell/src/main.rs` (+8 lines: 2 enum variants, 2 scancode mappings, 2 handlers)

## Not In Scope (future)

- Visual placeholder rendering (next phase)
- `servers/quil/` binary (after placeholder)
- Text editor / code mode / Sex Mode implementations
- Filesystem or document authority
- Keyboard shortcut customization (see SCAN 8 in QUIL_SURFACE_STUB_V1.md)

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Add F8/F9 keyboard bindings for Linen and Quil toggle | SURFACE_ACTION_BINDINGS_V1 |
