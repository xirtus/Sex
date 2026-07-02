# L3A: Fix Linen Row Visual Height

**Status:** Complete  
**Phase:** L3A — resolves L3 audit WARN  
**Files changed:** `servers/silk-shell/src/main.rs`  
**Build:** `[SEXOS ENTRYPOINT] success`

---

## Root Cause (L3 WARN)

Default Linen surface height was 150px. Required height:

```
LINEN_LIST_HEADER_H              = 28px
LINEN_LIST_ROW_RECTS × (ROW_H + ROW_GAP)
= 7 × (24 + 2)                   = 182px
margin                            = 10px
──────────────────────────────────────
Total                             = 220px
```

Rows 4-6 (y=132, 158, 184) exceeded surface height. Sexdisplay's `fill_sy` clamp
squashed them to `max_sy = slot.h - sh` — no corruption, but visual overlap.

## Fix

Added `LINEN_SURFACE_VISUAL_H` const derived directly from the row constants:

```rust
const LINEN_SURFACE_VISUAL_H: u32 =
    LINEN_LIST_HEADER_H + LINEN_LIST_ROW_RECTS as u32 * (LINEN_LIST_ROW_H + LINEN_LIST_ROW_GAP) + 10;
```

Evaluates at compile time to `28 + 7 * 26 + 10 = 220`.

Changed Linen surface default height:
```rust
// Before:
static mut SURFACE_200_H: u32 = 150;
// After:
static mut SURFACE_200_H: u32 = LINEN_SURFACE_VISUAL_H;
```

`SURFACE_200_H` is the Linen surface (surface ID 200) height only. No other surface changed.

## Additional Fixes (pre-existing breakage from earlier session)

Two incomplete Atlas changes from a previous session broke the build:

1. **`ATLAS_CARD_ACTIVE_RIM_COLOR` deleted but still referenced (lines 4619, 4622, 4625, 4628)**  
   Restored: `const ATLAS_CARD_ACTIVE_RIM_COLOR: u32 = 0x004090c0;`

2. **`SceneDescriptor` gained `accent` and `pinned` fields but init at line 4113 was not updated**  
   Fixed: added `accent: 0, pinned: false` to the struct literal.

Neither fix changes behavior — both were compile-time breakage from partial prior work.

## L3 Audit: PASS

| Finding | Verdict |
|---------|---------|
| Linen rows 4-6 clipping | **FIXED** — surface height = 220px |
| Encoding correctness (bit-56 rect_index) | PASS |
| Stale rects with fill_count | PASS |
| Atlas overlay backward compat | PASS |
| Proof marker budget | PASS |
| STOP FIRST check | PASS |

L3 audit closed as **PASS**.
