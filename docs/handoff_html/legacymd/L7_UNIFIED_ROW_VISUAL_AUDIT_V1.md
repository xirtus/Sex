# L7: Unified Row Visual Audit

**Status:** Complete
**Date:** 2026-05-05
**Purpose:** Verify Linen + Quil + Command Palette row visuals against
actual renderer state. Determine whether sexdisplay multi-rect is truly
active or only shell-side packing is staged.

## Verdict

```
╔══════════════════════════════════════════════════════════════════╗
║                    PASS_L_ROW_VISUALS                            ║
╠══════════════════════════════════════════════════════════════════╣
║ Renderer multi-rect:     ACTIVE (MAX_RECTS=8 deployed)          ║
║ Shell/display packing:   MATCH (bits 56-59, validated)          ║
║ Color isolation:         CLEAN (& 0x00FF_FFFF mask)             ║
║ All 3 surface producers: CONFORMANT                              ║
║ Build:                  PASSES (ISO produced)                    ║
║ Forbidden areas:         CLEAN                                   ║
║ STOP FIRST triggers:     NONE                                    ║
╚══════════════════════════════════════════════════════════════════╝
```

## 1. Renderer Implementation Status: ACTIVE

**Sexdisplay already has multi-rect arrays deployed.** This is NOT forward-compatible
packing only — the renderer actively stores and composites up to 8 independent
fill rects per surface.

### Sexdisplay Surface Struct (`servers/sexdisplay/src/main.rs` lines 28-51)

```rust
const MAX_RECTS: usize = 8;

struct Surface {
    // ... geometry + chrome fields ...
    fill_count: u8,              // Number of active fill rects (0-8)
    fill_sx: [i32; MAX_RECTS],   // Array of rect X positions
    fill_sy: [i32; MAX_RECTS],   // Array of rect Y positions
    fill_sw: [u32; MAX_RECTS],   // Array of rect widths
    fill_sh: [u32; MAX_RECTS],   // Array of rect heights
    fill_color: [u32; MAX_RECTS],// Array of rect colors
}
```

Six scalar fill fields replaced by `MAX_RECTS=8` arrays. `fill_count` tracks
the highest set index + 1 (0 = no rects). Arrays are zero-initialized.

### 0xEF Handler (lines 1086-1140)

| Step | Operation | Code Reference |
|------|-----------|----------------|
| 1 | Extract `rect_index` from bits 56-59 of arg2 | `let rect_index = ((msg.arg2 >> 56) & 0xF) as usize;` (line 1100) |
| 2 | Validate `rect_index < MAX_RECTS` | Reject with `[display.fill_rect.reject.index]` (line 1117-1120) |
| 3 | Bounds clamp per rect (same as single-rect) | `sw.min(slot.w)`, `sx.clamp(0, max_sx)` (lines 1123-1130) |
| 4 | Write to array slot `[rect_index]` | `slot.fill_sx[rect_index] = fill_sx` etc. (lines 1132-1136) |
| 5 | Update `fill_count` to max | `fill_count = max(fill_count, rect_index + 1)` (lines 1137-1138) |
| 6 | Emit `[display.fill_rect.set]` proof marker | (line 1140) |
| 7 | Call `redraw_surface_area()` | (line 1142) |

### Composite: fill_rect_color (lines 312-326)

```rust
fn fill_rect_color(surf: &Surface, x: usize, y: usize, base_color: u32) -> u32 {
    if surf.fill_count == 0 { return base_color; }
    let mut c = base_color;
    for i in 0..surf.fill_count as usize {
        if surf.fill_sw[i] == 0 || surf.fill_sh[i] == 0 { continue; }
        if lx >= surf.fill_sx[i] && lx < surf.fill_sx[i] + surf.fill_sw[i] as i32
            && ly >= surf.fill_sy[i] && ly < surf.fill_sy[i] + surf.fill_sh[i] as i32
        {
            c = surf.fill_color[i];  // last match wins (painter's algorithm)
        }
    }
    c
}
```

Iterates all active rects. Later rects overdraw earlier ones. This is correct
painter's algorithm. Performance is trivial (max 8 iterations per pixel).

**Verdict: RENDERER_MULTI_RECT_ACTIVE. Not staged. Truly deployed.**

## 2. Canonical rect_index Packing: bits 56-59

### Canonical Format

The `0xEF` opcode `arg2` uses this bit layout:

```
bit 63  56 55        32 31    16 15     0
┌────────┬──────────────┬──────────┬──────┐
│ rect   │   color      │   sh     │  sw  │
│ index  │   (24-bit)   │  (16-bit)│(16bit)│
│ (4bits)│              │          │      │
└────────┴──────────────┴──────────┴──────┘
```

- **bits 0-15** (0xFFFF): `sw` — rect width
- **bits 16-31** (0xFFFF0000): `sh` — rect height
- **bits 32-55** (0xFFFFFF0000000000): `color` — 24-bit RGB (lower 24 bits)
- **bits 56-59** (0x0F00000000000000): `rect_index` — fill rect slot (0-7)
- **bits 60-63**: Reserved (must be zero)

### Shell Packing (all 3 producers)

```rust
arg2 = (rect_index << 56)
     | ((color as u64) << 32)
     | ((sh as u64) << 16)
     | (sw as u64)
```

### Sexdisplay Decode (line 1099-1100)

```rust
let color = ((msg.arg2 >> 32) & 0x00FF_FFFF) as u32;
let rect_index = ((msg.arg2 >> 56) & 0xF) as usize;
```

**Key: Color isolation is clean.** The `& 0x00FF_FFFF` mask on line 1099
discards bits 24-31 of the shifted value (which correspond to bits 56-63
of the raw arg2). This means rect_index bits are **masked out** of the
color — no color corruption.

### Packing Verification

| arg2 bit range | Shell sends | Sexdisplay reads | Match? |
|---------------|-------------|------------------|--------|
| 0-15 (sw) | `w` | `msg.arg2 & 0xFFFF` | ✅ |
| 16-31 (sh) | `sh << 16` | `(msg.arg2 >> 16) & 0xFFFF` | ✅ |
| 32-55 (color) | `color << 32` | `(msg.arg2 >> 32) & 0x00FF_FFFF` | ✅ |
| 56-59 (rect_index) | `rect_index << 56` | `(msg.arg2 >> 56) & 0xF` | ✅ |

**Verdict: SHELL_DISPLAY_PACKING_MATCH. Fully compatible.**

### Design Document Correction

The L1 document (`docs/handoff/L1_MULTI_RECT_DISPLAY_STOP_FIRST_DESIGN_V1.md`)
specified rect_index in bits 24-27 of arg2. The actual implementation uses
bits 56-59, which is correct and does not conflict with sw/sh decoding.

**Recommendation:** Update L1 doc to reference the canonical bits 56-59 format.
(LOW severity — document correction only.)

## 3. Shell Producer Conformance Table

### rect_index Usage

| Producer | Surface | Header index | Row indices | Row rects | Total rects |
|----------|---------|-------------|-------------|-----------|-------------|
| Linen | 200 | 0 (implicit, no <<56) | 1-7 | `LINEN_LIST_ROW_RECTS=7` | 8 |
| Quil | 201 | 0 (implicit, no <<56) | 1-7 | `QUIL_LIST_ROW_RECTS=7` | 8 |
| Command palette | 0x98 | 0 (implicit, no <<56) | 1-5 | `PALETTE_LIST_ROW_RECTS=5` | 6 |

All three pack rect_index into bits 56-59 using the exact same formula:
```rust
let rect_index = (rows_emitted as u64 + 1) & 0xF;
arg2 = (rect_index << 56) | ((color as u64) << 32) | ((sh as u64) << 16) | w as u64;
```

### Header vs Row Pattern

- **Header** (rect_index=0): `pdx_call(0xEF, sid, 0, color<<32 | sh<<16 | w)`
  — no explicit rect_index packing; bits 56-63 default to zero → rect_index=0.
- **Rows** (rect_index=1..N): `pdx_call(0xEF, sid, row_y<<32, rect_index<<56 | color<<32 | sh<<16 | w)`
  — explicit rect_index packing into bits 56-59.

### Row Color Selection

| Producer | Header color | Row color (selected) | Row color (unselected) |
|----------|-------------|---------------------|------------------------|
| Linen | `linen_selected_object_accent()` | Same as header | `linen_kind_color(obj.kind)` |
| Quil | `QUIL_LIST_HEADER_COLOR` (fixed) | N/A (no selection) | `quil_buffer_kind_color(buf.kind)` |
| Command palette | `command_palette_selected_accent()` | Same as header | `command_kind_color(cmd)` (muted) |

## 4. Surface Geometry / Clipping Table

| Surface | Width | Height | Visual Top | Visual Height | Fits? |
|---------|-------|--------|------------|---------------|-------|
| Linen (200) | 300 | 220 | 0 | 28 + 7(24+2) = 210 | ✅ (10px padding) |
| Quil (201) | 640 | 480 | 0 | 28 + 7(24+2) = 210 | ✅ (270px padding) |
| Command palette (0x98) | 480 | 240 | 0 | 28 + 5(24+2) = 158 | ✅ (82px padding) |

All surface heights are sufficient. No clipping risk. Sexdisplay bounds-clamp
per rect (`sw.min(slot.w)`, `fill_sx.clamp(0, max_sx)`) as a safety net.

## 5. Boundary Check

| Boundary | Status | Evidence |
|----------|--------|----------|
| New opcodes | ✅ NONE | Reuses existing 0xEF only |
| sex-pdx ABI edits | ✅ NONE | No changes to `crates/sex-pdx/` |
| Kernel edits | ✅ NONE | No changes to `kernel/` |
| Renderer policy ownership | ✅ INTAKT | Sexdisplay composites rects without interpreting semantics |
| Framebuffer bounds checks | ✅ PRESERVED | Double-bounded: `sw.min(slot.w)` + `sx.clamp(0, max_sx)` |
| MAX_RECTS fixed upper bound | ✅ 8 | Compile-time constant, no heap |
| Authorization gate | ✅ PRESERVED | `caller_pd == owner_pd` check per 0xEF call |

## 6. Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| L1 doc mentions bits 24-27; actual uses bits 56-59 | LOW | Doc-only mismatch; no behavioral impact |
| Linen surface height overridden to 220px; other surfaces use boot default | LOW | Intentional; documented in L3A |
| Quil/Command palette have excess vertical padding | LOW | Visual waste, not a bug |
| All 3 producers pack rect_index identically but have no shared helper | LOW | Could extract shared fn, not required |

No new risks from the multi-rect deployment.

## 7. Exact Next Safest Step

All three row visual consumers are deployed and verified. Next feature work
options (in priority order):

1. **Bell event real implementation** (stub → real if Collar/Mesh ready) — now that
   Bell placeholder surface has multi-rect visuals for event lists
2. **Mesh diagnostic fact rows** — uses same multi-rect pattern for link visualization
3. **Collar grant status display** — uses same multi-rect pattern
4. **L1 doc correction** — update bits 24-27 → bits 56-59 (30-second docs fix)
5. **Shared row-render helper** — extract common `emit_row_rects()` pattern
   (cosmetic refactor, no behavior change)

## Summary

The multi-rect display system is **fully live**, not staged:
- Sexdisplay already stores and composites up to 8 rects per surface
- Shell packing and sexdisplay decode match exactly (bits 56-59)
- Color isolation is clean (`& 0x00FF_FFFF` mask)
- All three surfaces (Linen, Quil, Command Palette) use the identical pattern
- Each producer stays within `MAX_RECTS=8` budget
- No new opcodes, no ABI changes, no kernel edits, no renderer policy drift
