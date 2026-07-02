# APP_SURFACE_LAUNCH_CONTRACT_V1

**Status:** Implemented
**Date:** 2026-05-06
**Files changed:** 2 (+160 / -0 lines)

---

## Route Chosen

A userland app-like PD requests one surface via IPC to silk-shell. Silk-shell validates, creates a `ShellFrame` + `ShellTab`, registers lifecycle, and upserts on sexdisplay via 0xEC. The app PD never writes framebuffer — focus ownership remains shell-only.

### Opcode

`OP_APP_SURFACE_REQ = 0xFA` — sent to silk-shell via `SLOT_SHELL` (slot 6).

- `arg0 = surface_id` (must be >= 200, non-zero)
- `arg1 = title_id` (opaque u64 for tab title, must be non-zero)
- `arg2 = reserved` (future: packed geometry)

### Validation gates (reject with serial marker)

1. `surface_id == 0` → `[shell.app_surface.reject] reason=zero_surface_id`
2. `title_id == 0` → `[shell.app_surface.reject] reason=zero_title_id`
3. Already registered in lifecycle → `[shell.app_surface.reject] reason=already_registered`
4. `surface_id < 200` (OS/reserved range) → `[shell.app_surface.reject] reason=reserved_range`
5. No free frame slot → `[shell.app_surface.reject] reason=no_frame_slot`

### On accept

- Allocates dynamic frame ID (starts at 10 to avoid boot frame collision)
- Creates `ShellFrame` with one `ShellTab` containing `surface_id` + `title_id`
- Registers lifecycle state as `Visible`
- Upserts on sexdisplay via `pdx_call(SLOT_DISPLAY, 0xEC, surface_id, (y<<32)|x, (h<<32)|w)`
- Re-tiles and sets focus to new surface
- Emits `[shell.app_surface.accept] sid=X title_id=X frame=X caller=X`

### Hover safety guard

`clear_hover_if_dead()` added to 7 call sites alongside existing `clear_focus_if_dead()` / `clear_drag_if_dead()`. If `HOVERED_FRAME_ID` references a dead or tombstoned surface, hover is cleared immediately. Emits `[shell.hover.clear.dead]` with frame and surface id.

---

## Proof Markers (synthetic, gated by `SEXOS_APP_SURFACE_REQ_PROOF=1`)

Four proof stages run at boot before the main listen loop:

| Stage | Call | Expected | Marker |
|-------|------|----------|--------|
| 0 | `handle_app_surface_req(300, 42, 0)` | accepted (valid) | `[shell.app_surface.proof] stage=0 accepted=true` |
| 1 | `handle_app_surface_req(0, 42, 0)` | rejected (zero sid) | `[shell.app_surface.proof] stage=1 accepted=false` |
| 2 | `handle_app_surface_req(301, 0, 0)` | rejected (zero title) | `[shell.app_surface.proof] stage=2 accepted=false` |
| 3 | `handle_app_surface_req(300, 99, 0)` | rejected (duplicate sid) | `[shell.app_surface.proof] stage=3 accepted=false` |

---

## Build / Runtime

- Build: `./scripts/entrypoint_build.sh` — PASS (no regressions)
- No kernel edits. No ABI changes. No renderer primitives.
- Feature-gated: default-off, zero behavior change when env var unset.

## Remaining Risks

1. **No app PD exists yet**: The opcode is defined but no userland app currently sends `0xFA`. Synthetic proof exercises the handler, but end-to-end app launch is untested.
2. **No error reply**: If validation fails, the caller gets no explicit error code — only silent reject. A future V2 should send a reply with error reason.
3. **Frame geometry hardcoded**: `normal_x=200, normal_y=100, normal_w=600, normal_h=400`. Future V2 should accept packed geometry from the caller.
4. **No surface destruction contract**: When the app PD dies, lifecycle tombstoning applies, but there's no explicit "app surface closed" notification back to the caller.

## Files Changed

```
servers/silk-shell/src/lib.rs  +1   (OP_APP_SURFACE_REQ constant)
servers/silk-shell/src/main.rs +159 (handler, proof, hover guard)
```

No sex-pdx ABI changes. No kernel edits. No renderer primitives.
