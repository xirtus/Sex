# SILK_MANAGES_LINEN_PLACEHOLDER_V1

Date: 2026-05-07
Status: LANDED
Requires: SURFACE_CLIENT_ID_AUTH_V1 (landed)

## Files Changed

- `servers/silk-shell/src/main.rs` — two edits

No sexdisplay changes. No kernel changes. No sex-pdx changes.

## Root Cause Found

`is_focusable_surface()` at line ~4593 did not include `SURFACE_ID_LINEN`. This caused
`try_set_focus(200)` to fail immediately with `reason=nonfocusable` even though:
- `surface_is_alive(200)` = true (hardcoded)
- `surface_is_lifecycle_focusable(200)` = true (Visible at boot)
- Focus200 action (key 5 → scancode 0x06) was already wired
- Linen was already in `access_emit_shell_nodes` (Tab cycle node 4)
- OP_REGISTER_WM registration means shell can send 0xED on any surface

## Edits

### 1. `is_focusable_surface` — add Linen
```
fn is_focusable_surface(sid: u64) -> bool {
    sid == SURFACE_ID_APP || sid == SURFACE_ID_STATIC
    || sid == SURFACE_ID_TEST3 || sid == SURFACE_ID_TEST4
    || sid == SURFACE_ID_LINEN  // client surface managed as WM placeholder
    || app_surface_spec(sid).map_or(false, |s| s.focusable)
}
```

### 2. Arrow key move handler — add Linen branch
After the TEST4 branch, new branch:
```
} else if focused == SURFACE_ID_LINEN && value == 1 {
    match scancode {
        0x4B => { SURFACE_200_X -= step; mutated = true; }
        0x4D => { SURFACE_200_X += step; mutated = true; }
        0x48 => { SURFACE_200_Y -= step; mutated = true; }
        0x50 => { SURFACE_200_Y += step; mutated = true; }
        _ => {}
    }
    if mutated {
        let (cx, cy) = clamp_position(SURFACE_200_X, SURFACE_200_Y, SURFACE_200_W, SURFACE_200_H);
        SURFACE_200_X = cx; SURFACE_200_Y = cy;
        serial_println!("[shell.linen.move] x={} y={}", SURFACE_200_X, SURFACE_200_Y);
    }
}
```

Position propagates to sexdisplay via `snap_capture_layout → OP_SURFACE_UPDATE (0xEB)`.
Shell has WM auth so 0xEB on Linen surface succeeds.

## What Was Already Wired (No Changes Needed)

| Feature | Location | Status |
|---------|----------|--------|
| Key 5 → Focus200 | scancode 0x06 → SurfaceAction::Focus200 | ✅ existing |
| Focus200 handler | ~line 11153: try_set_focus(SURFACE_ID_LINEN) | ✅ existing |
| Tab cycle includes Linen | access_emit_shell_nodes node 4 | ✅ existing |
| surface_is_alive(200) | hardcoded true | ✅ existing |
| lifecycle_register(200, Visible) | lifecycle_init_all() | ✅ existing |
| 0xEB on Linen via snap_capture_layout | snap_capture_layout line ~4160 | ✅ existing |
| F2 DestroyFocused skips Linen | only handles 100/101/102/103 | ✅ safe |
| R ResetAll skips Linen | only handles 100/101/102/103 | ✅ safe |

## Runtime Proof Markers

Expected on key 5 press:
```
[shell.focus.set] id=200
[shell.interact.focus] sid=200
[focus.ref.commit] id=200
[silk-shell] Focus switched to surface 200
```

Expected on Tab cycle reaching Linen:
```
[access.action.focus_next] from=<prev> to=200 role=AppPlaceholder label=Linen
```

Expected on arrow key with Linen focused:
```
[shell.linen.move] x=<new_x> y=<new_y>
```

WM registration (from V1):
```
[sexdisplay.auth.wm.register] caller=<N> ok=1
```

## Safety Invariants

- Shell does NOT call 0xEE on surface 200 (destroy blocked by V1 policy, also DestroyFocused has no Linen branch)
- Linen PD still owns surface 200 (owner_pd set at Linen's first 0xEC call)
- Shell's 0xEB on 200 succeeds only because shell is registered WM (V1 auth)
- If Linen surface 200 does not yet exist in sexdisplay (Linen not booted yet), 0xEB is a no-op (sexdisplay ignores unknown sid)
- focus/move fail soft if surface not alive (surface_is_alive check in try_set_focus)

## Next

LINEN_UI_STATIC_V1
