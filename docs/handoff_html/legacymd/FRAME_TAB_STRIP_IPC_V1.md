# FRAME_TAB_STRIP_IPC_V1

## Status

Implemented (2026-05-04). Tab metadata IPC from silk-shell to sexdisplay via direct 0xFD opcode. Colored tab blocks rendered in the top rim band of focused surfaces. No kernel/ABI changes. No text rendering. No tab switching.

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| Tab metadata flows from shell to display | `OP_SURFACE_TAB_INFO = 0xFD` opcode, direct silk-shell → sexdisplay | `[shell.frame.tab.info.send]` |
| Sexdisplay stores tab info | `Surface.tab_count`, `Surface.active_tab` fields updated on 0xFD | `[sexdisplay.surface.tab.info]` |
| Colored tab blocks rendered in top rim | `composite_pixel()` Pass 2, after lights, before rim fallback | Visual only |
| Active tab distinct from inactive | Active = `FOCUS_SURFACE_COLOR` (cyan), Inactive = `0x006080B0` (dim cyan) | Visual only |
| Frame Lights still take priority | Tab strip check comes AFTER light checks in `composite_pixel()` | Lights visible |
| Right rim preserved | Tab strip check requires `lx < rim_right` | Rim visible on right edge |
| Light exclusion zone preserved | Tab strip starts at `lx >= TAB_STRIP_LIGHT_EXCLUSION_PX (20)` | Lights zone excluded |
| Non-frame surfaces unaffected | `tab_count` defaults to 0, no tab blocks drawn | Existing surfaces unchanged |
| No kernel/ABI changes | Userland opcode only, no syscall/module changes | Build passes |
| No ABI_VERSION change | New opcode orthogonal to SilkBar model | `SILKBAR_ABI_VERSION` unchanged |

---

## IPC Protocol

### Data Flow

```
silk-shell (PDX 6)                         sexdisplay (PDX 4)
    │                                           │
    │ 0xFD (OP_SURFACE_TAB_INFO)                 │
    │  arg0 = surface_id                         │
    │  arg1 = tab_count                          │
    │  arg2 = active_tab                         │
    │ ──────────────────────────────────────────→│
    │                                           │
    │ (sent after boot surface create + focus)  │
    │ (future: sent on tab switch)              │
    │                                           │
    │                                           │ store in Surface.tab_count
    │                                           │ store in Surface.active_tab
    │                                           │ redraw_surface_area()
    │                                           │ render tab blocks in
    │                                           │ composite_pixel() Pass 2
```

### Opcode

```
crates/sex-pdx/src/lib.rs:
  pub const OP_SURFACE_TAB_INFO: u64 = 0xFD;
```

### Payload

| Field | Register | Type | Validation |
|-------|----------|------|------------|
| surface_id | arg0 | u64 | Reject if 0. Silently ignore if no matching active surface. |
| tab_count | arg1 | u64 | Clamped to max 8 (MAX_TABS_PER_FRAME). |
| active_tab | arg2 | u64 | If tab_count > 0, clamped to tab_count-1. If tab_count == 0, set to 0. |

### Opcode Dispatch (sexdisplay)

Inserted between 0xEF (fill rect) and `_ =>` catch-all. No caller authentication (follows 0xED focus pattern — chrome metadata is compositor state).

---

## Constants Added

### crates/sex-pdx/src/lib.rs

```rust
pub const OP_SURFACE_TAB_INFO: u64 = 0xFD;
```

### servers/sexdisplay/src/main.rs

```rust
struct Surface {
    // ...existing fields...
    active: bool,
    tab_count: u8,      // NEW
    active_tab: u8,      // NEW
    // ...fill rect fields...
}

const TAB_STRIP_LIGHT_EXCLUSION_PX: usize = 20;
const TAB_ACTIVE_COLOR: u32 = FOCUS_SURFACE_COLOR;   // 0x00A8E0FF
const TAB_INACTIVE_COLOR: u32 = 0x006080B0;           // dimmed cyan
```

---

## Rendering Geometry

### Top Rim Band Layout (y < FRAME_RIM_PX, focused surface only)

```
lx=0    2    6    8    12   14   18 20                      rim_right=(sw-4)
         │    │    │    │    │    │  │                           │
┌────────┴────┴────┴────┴────┴────┴──┴───────────────────────────┘
│ CLOSE  │MIN │ZOOM│  GAP  │  TAB STRIP (active=cyan, inactive=dim)
│ (red)  │(yel│(grn)│      │  equal-width colored blocks
└────────┴────┴────┴───────┴────────────────────────────────────────
 ← 2px   →4px→2→4→2→4→2→   ←─ 20px exclusion →← available width →
```

### Tab Slot Computation (in composite_pixel)

```rust
if surf.tab_count > 0
    && lx >= TAB_STRIP_LIGHT_EXCLUSION_PX  // 20px from surface left edge
    && lx < rim_right                       // stop before right rim
{
    let available = rim_right - TAB_STRIP_LIGHT_EXCLUSION_PX;
    let slot_w = available / surf.tab_count as usize;
    if slot_w > 0 {
        let tab_idx = (lx - TAB_STRIP_LIGHT_EXCLUSION_PX) / slot_w;
        if tab_idx == surf.active_tab as usize {
            c = TAB_ACTIVE_COLOR;
        } else {
            c = TAB_INACTIVE_COLOR;
        }
    }
}
```

For boot geometry (sw=800): `available = (800-4) - 20 = 776`, `slot_w = 776/1 = 776`.

---

## Files Changed

| File | Changes |
|------|---------|
| `crates/sex-pdx/src/lib.rs` | Added `OP_SURFACE_TAB_INFO = 0xFD` constant |
| `servers/sexdisplay/src/main.rs` | Extended `Surface` with `tab_count: u8, active_tab: u8`. Updated `SURFACE_EMPTY` + 2 create-site initializers. Added `TAB_STRIP_LIGHT_EXCLUSION_PX`, `TAB_ACTIVE_COLOR`, `TAB_INACTIVE_COLOR` constants. Added 0xFD opcode handler with budget marker `[sexdisplay.surface.tab.info]`. Added tab strip rendering in `composite_pixel()` Pass 2 (top rim, after lights, before rim fallback). |
| `servers/silk-shell/src/main.rs` | Added `OP_SURFACE_TAB_INFO` to sex-pdx import. Added `send_frame_tab_info()` helper with budget marker `[shell.frame.tab.info.send]`. Called `send_frame_tab_info(1)` after boot surface create + focus set. |
| `sexos_build_spec.toml` | Updated `abi_version_hash` to reflect sex-pdx change. |

### Files NOT Modified

Kernel, PDX ABI, silkbar, silkbar-model, sexusb, sexinput, framebuffer path — all untouched.

---

## Diagnostic Markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.tab.info.send] frame=N surface=N tabs=N active=N` | 8 | Silk-shell sends tab metadata to sexdisplay at boot |
| `[sexdisplay.surface.tab.info] surface=N tabs=N active=N` | 8 | Sexdisplay receives and stores tab metadata |

Pre-existing markers that must still fire:

| Marker | Status |
|--------|--------|
| `[shell.frame.tab.model]` | Tab strip model proof (boot) |
| `[shell.frame.light.model]` | Frame Lights model proof (boot) |
| `[shell.frame.light.close/minimize/zoom]` | Light clicks still work |
| `[shell.frame.zoom/unzoom]` | ZOOM light toggle works |
| `[shell.frame.minimize]` | MINIMIZE light works |
| `[shell.drag.start/move/end]` | Rim drag works |
| `[shell.hit_target.chrome]` | Chrome hit targets produced |
| `[sexdisplay.cursor.surface.update]` | Cursor updates |

---

## Build

```bash
./scripts/entrypoint_build.sh
```

Both default and synthetic build pass. No new warning types. Pre-existing warnings unchanged.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-tab-strip-ipc-v1.log

for m in \
  shell.frame.tab.info.send \
  sexdisplay.surface.tab.info \
  shell.frame.tab.model \
  shell.frame.light.hover \
  shell.frame.zoom \
  shell.frame.minimize \
  shell.frame.light.close \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end
do
  printf "%-46s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-tab-strip-ipc-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-tab-strip-ipc-v1.log
```

### Pass criteria

- Default build passes ✅
- Synthetic build passes (optional) ✅
- `[shell.frame.tab.info.send]` fires at boot with `frame=1 surface=100 tabs=1 active=0`
- `[sexdisplay.surface.tab.info]` fires at boot with `surface=100 tabs=1 active=0`
- Focused surface shows colored tab block in top rim band (cyan, full width)
- Frame Lights (red/yellow/green) still visible and override tab blocks
- Rim still visible on right edge and bottom
- Close/minimize/zoom still work via their respective lights
- Rim drag still works on non-light, non-tab rim clicks
- No panic/#PF/#GP
- No kernel edits confirmed
- No syscall ABI changes confirmed

---

## Remaining Risks

- **No text labels**: Tab blocks are colored rectangles. No text/title rendering. Tab identity is not visible to the user.
- **Single-tab V1**: Only 1 tab exists in V1 boot configuration. Tab strip shows a single colored block spanning the entire available width. Multi-tab rendering untested.
- **Tab switching not implemented**: The shell has tab_count=1 and never sends updated 0xFD after boot. Tab switching (changing active_tab) is deferred.
- **No hover feedback**: Hovering over the tab strip produces `[shell.drag.skip.chrome]` but no visible change. Tab block color remains the same.
- **0xFD redraw trigger**: Tab info update triggers a full `redraw_surface_area()`. For V1 this is fine (single boot call). If tab info is sent frequently in the future, consider more targeted redraw.

---

## No-Kernel-Change Confirmation

Confirmed: All changes are userland only.

| Area | Change | Kernel Impact |
|------|--------|---------------|
| `crates/sex-pdx/src/lib.rs` | Added const `OP_SURFACE_TAB_INFO = 0xFD` | None — userland constant, no kernel dispatch |
| `servers/sexdisplay/src/main.rs` | New match arm `0xFD`, Surface fields, composite_pixel rendering | None — opcode already dispatched by userland `pdx_listen_raw` |
| `servers/silk-shell/src/main.rs` | Calls `pdx_call(SLOT_DISPLAY, 0xFD, ...)` | None — uses existing `pdx_call` syscall path |
| `sexos_build_spec.toml` | Updated ABI hash | None — spec metadata only |

The 0xFD opcode is handled entirely in userland. The kernel's PDX dispatch matches by `type_id` and routes to the target slot's listener. The target slot (SLOT_DISPLAY = 5) is sexdisplay, which receives all opcodes via `pdx_listen_raw(0)` and dispatches by `msg.type_id` in a userland `match` block. No kernel change is required or made.

---

## Next Recommended Phase

### FRAME_TAB_SWITCH_PLAN_V1

Design for tab switching: clicking a tab block in the tab strip switches the active tab within a frame. Requires:
- Tab click detection in `click_hit_test_and_focus()` (already returns `FrameChrome { kind: FRAME_CHROME_TAB_STRIP }`)
- Tab index extraction from click position (use `frame_tab_at()` which already exists)
- `ShellFrame.active_tab` update
- Sexdisplay surface focus switch to the clicked tab's surface_id
- Updated 0xFD opcode send with new active_tab
- No text labels, no tab strip UI changes
