# SHELL_FONT_GLYPH_HELPER_SPEC_V1

**Status:** PASS REVIEW ONLY — Existing sexdisplay OP_TEXT_DRAW (0xFB) already provides glyph rendering.
**Date:** 2026-05-16

---

## 0. Key Finding: No new helper needed

Sexdisplay already handles `OP_TEXT_DRAW` (0xFB) for glyph text rendering on any surface. Quil uses it in `draw_text_lines()`. The shell can use the exact same protocol for WebStub.

---

## 1. Option Comparison Table

| Option | Safe? | Notes |
|--------|-------|-------|
| **A. Use existing sexdisplay OP_TEXT_DRAW** | ✅ **YES** | Protocol exists. Shell just needs to format bytes and call `pdx_call(SLOT_DISPLAY, 0xFB, sid, packed, offset_and_color)`. Quil already does this. |
| B. Extract Quil helper into shared crate | ⚠️ Unnecessary | Not needed — OP_TEXT_DRAW is the shared protocol. |
| C. Sexdisplay text primitive | ✅ Already exists | 0xFB handler at sexdisplay line 2083. |
| D. New PDX text protocol | ❌ Not needed | OP_TEXT_DRAW already exists. |
| E. Keep colored bands only | ⚠️ Acceptable fallback | Current state (102 gates). |

---

## 2. Recommended Path: **A — Use existing OP_TEXT_DRAW**

### Protocol (already exists in sexdisplay)

```
pdx_call(SLOT_DISPLAY, 0xFB, surface_id, packed_bytes, offset_and_color)
  arg0: surface_id (e.g., SURFACE_ID_BROWSER = 205)
  arg1: up to 8 ASCII bytes packed little-endian
  arg2: byte_offset (bits 0-7) | char_count (bits 8-11) | text_color (bits 32-63)
```

Sexdisplay handler at line 2083 validates surface ownership, then renders glyphs using the 5×7 ASCII font.

---

## 3. Shell-Side Helper API (minimal, no_std)

```rust
/// Render bounded ASCII text on a surface via OP_TEXT_DRAW.
/// Returns (bytes_sent, ok) — caller owns surface bounds.
fn shell_draw_text(surface_id: u64, text: &[u8], color: u64) -> (usize, bool) {
    let max_bytes = text.len().min(256); // bounded
    let mut offset: usize = 0;
    while offset < max_bytes {
        let chunk = (max_bytes - offset).min(8);
        let mut packed: u64 = 0;
        for i in 0..chunk {
            packed |= (text[offset + i] as u64) << (i * 8);
        }
        let arg2 = (offset as u64 & 0xFF)
            | ((chunk as u64 & 0xF) << 8)
            | (color << 32);
        pdx_call(SLOT_DISPLAY, 0xFB, surface_id, packed, arg2);
        offset += chunk;
    }
    (offset, offset > 0)
}
```

This is ~15 lines, no_std, no heap, reuses existing protocol. Directly follows Quil's pattern.

---

## 4. Safety Invariants

| Invariant | How |
|-----------|-----|
| Bounded text length | max 256 bytes |
| Surface ownership | Sexdisplay validates caller owns surface |
| Bounds checking | Sexdisplay clips glyphs to surface rect |
| Unknown chars | 5×7 font maps 32-126; others render as space |
| No font loading | Static 5×7 ASCII bitmap in sexdisplay |
| No alpha/blur/shadow | OP_TEXT_DRAW uses flat color |
| No renderer policy change | Shell calls existing protocol, sexdisplay renders |

---

## 5. Phase Ladder

| Phase | What |
|-------|------|
| 0 | This spec |
| 1 | `shell_draw_text()` helper in silk-shell (~15 lines) |
| 2 | WebStub text glyph render using helper |
| 3 | Quil regression gate (ensure Quil text still works) |
| 4 | URL intent visible status text |
| 5 | General surface text helper for any shell-owned surface |

---

## 6. STOP FIRST Boundaries

| Boundary | Status |
|----------|--------|
| >150 line extraction | ❌ N/A — helper is ~15 lines |
| Changes to text buffer semantics | ❌ No |
| Framebuffer ownership | ❌ No change |
| New sexdisplay protocol | ❌ OP_TEXT_DRAW already exists |
| Heap/std/libc/thread | ❌ None needed |
| Broad Quil refactor | ❌ No — Quil unaffected |

---

## 7. Next Prompt

**MISSION: SHELL_DRAW_TEXT_HELPER_V1**

Implement `shell_draw_text()` in silk-shell following the OP_TEXT_DRAW pattern used by Quil. ~15 lines. Then use it to render text on WebStub surface.

---

## 8. Handoff

```
docs/handoff/SHELL_FONT_GLYPH_HELPER_SPEC_V1.md
```

## 9. Commit

```bash
git add docs/handoff/SHELL_FONT_GLYPH_HELPER_SPEC_V1.md
git commit -m "docs(silk): shell font glyph helper spec V1"
```
