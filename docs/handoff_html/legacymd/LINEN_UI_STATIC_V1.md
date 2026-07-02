# LINEN_UI_STATIC_V1

Date: 2026-05-07
Status: LANDED
Requires: SURFACE_CLIENT_ID_AUTH_V1, SILK_MANAGES_LINEN_PLACEHOLDER_V1

## Linen Source Path

`servers/linen/src/main.rs` — Linen PD. Receives HID events, manages session objects.

**BUT:** Linen PD cannot draw to surface 200. Shell (silk-shell) creates surface 200 first
via 0xEC and becomes the owner. Linen PD's 0xEC/0xEF calls on surface 200 are rejected by
the V1 auth gate (owner_pd mismatch). All painting is done by silk-shell.

## Files Changed

- `servers/silk-shell/src/main.rs` — all rendering and selection logic

## Architecture

- silk-shell owns surface 200 (creates it first at boot via 0xEC)
- silk-shell paints surface 200 via 0xEF (fill rects) + 0xFB (text) + 0xFA (text clear)
- J/K key actions (SelectNextLinenObject/SelectPrevLinenObject) handled in silk-shell
- When Linen is focused, raw HID key events ALSO forwarded to Linen PD via OP_HID_EVENT
  (Linen PD can maintain internal state but cannot paint)

## New Additions in silk-shell

### Constants
```
const LINEN_UI_ROW_COUNT: usize = 5;
const LINEN_UI_ROW_COLORS: [u32; 5] = [
    0x003060A0,  // PROJECTS
    0x006040A0,  // SEX MICROKERNEL
    0x00306060,  // HANDOUTS
    0x00805030,  // HANDOFFS
    0x00204060,  // QUIL DRAFTS
];
static mut LINEN_UI_SELECTED: u8 = 0;
```

### Functions
- `linen_render_static_ui()` — paints 5 static rows using fill rects + text
- `linen_paint_surface()` — dispatcher: uses static UI when LINEN_OBJECTS empty, real list otherwise

### Selection Navigation (J/K)
SelectNextLinenObject: when no real objects, increments LINEN_UI_SELECTED (wraps 0..4)
SelectPrevLinenObject: when no real objects, decrements LINEN_UI_SELECTED (wraps 4..0)

## Visual Layout (surface 300×168)

| Rect index | Content | Position |
|-----------|---------|----------|
| 0 | Header band (selected row accent color) | y=0, h=28 |
| 1 | List background (dark slate) | y=28, h=130 |
| 2 | Selected row highlight (full width) | y=28+sel*26, h=24 |
| 3-7 | Per-row left accent bars (5px wide) | per row |

Text in surface text_buf (5×7 font, rendered at surf_x+8, surf_y+24):
- Offset 0: "LINEN" (title, in header band area)
- Offset 20: row 0 label
- Offset 40: row 1 label
- Offset 60: row 2 label
- Offset 80: row 3 label
- Offset 100: row 4 label

## Proof Markers

Boot:
```
[linen.ui.render] rows=5 selected=0
```

J/K navigation:
```
[linen.ui.select] index=N
[linen.ui.render] rows=5 selected=N
```

## Input Path Status

Input path EXISTS in silk-shell (J/K → SelectNextLinenObject/SelectPrevLinenObject).
Linen PD also receives HID events via SLOT_LINEN when focused (OP_HID_EVENT forwarding).
Linen PD's own handle_hid_event still runs but cannot paint (auth blocks 0xEF from Linen).

## Gap to LINEN_SEXFILES_LIST_V1

1. Linen PD needs a way to provide real object data to silk-shell for display
   - Options: Linen→Shell PDX reply to OP_LINEN_LIST_OBJECTS poll, or push notification
   - Currently shell has LINEN_OBJECTS array but no auto-populate mechanism
2. Text alignment with visual rows is approximate (9px line height vs 26px row height)
   - Acceptable for V1; would need multi-line text per row for proper alignment
3. Linen PD cannot paint surface 200 directly (arch constraint)
   - If Linen needs self-rendering: either shell delegates 0xEC first to Linen, or
     a new "paint grant" mechanism is needed in V2
4. Only 5 rows visible (LINEN_LIST_ACCENT_BARS=5); 7-row spec requires surface resize
