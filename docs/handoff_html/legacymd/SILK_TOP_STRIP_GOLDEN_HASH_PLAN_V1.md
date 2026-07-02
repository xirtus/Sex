# SILK_TOP_STRIP_GOLDEN_HASH_PLAN_V1

**Status:** PASS — DOCS-ONLY PLAN. No implementation, no source changes.
**Date:** 2026-05-16
**Depends on:** `COLLAR_GRANT_STATUS_STUB_V1.md` (96-gate baseline).
**Next:** Implementation phases 1–5.

---

## 0. PASS/FAIL

**PASS** — DOCS-ONLY PLAN. 0 gates, 0 faults. Design document for future deterministic top-strip verification.

---

## 1. Architecture Summary

### Goal

Replace visual-inspection-based QEMU GUI verification with a deterministic,
off-screen, hash-based golden-image test for the Silk top strip.

### Core Idea

```
fixed input vector → sexdisplay render (offscreen buffer) → ARGB rows → hash → compare to golden
```

No QEMU, no GUI, no human eyeball. A single `u64` hash proves the top strip
renders identically across builds.

### What Is Verified

| Component | What |
|-----------|------|
| SilkBar clock | Digit glyphs at correct positions, correct time |
| Workspace chips | Active/inactive colors, labels |
| Bell dot | Color (gold/amber/muted), count badge |
| Glass colors | Bar fill, chip fill, text color |
| Frame rim (top edge) | Color, intensity, bounds |
| Frame lights (top-left) | Red dim/disabled, yellow/green available |
| Top bar chrome | Background fill, divider, title text |

### What Is NOT Verified

| Excluded | Why |
|----------|-----|
| Full framebuffer | Only top strip (e.g., top 32–40 rows). Rest is scene-dependent. |
| Surface content | Quil/Linen/Spindle text changes per boot. Not hashable. |
| Animations | Pulse/blur/alpha compositing — excluded until deterministic. |
| Pointer/cursor | Position varies. |
| Network/browser content | Not applicable. |

---

## 2. Input Vector Table

A single frozen input struct fed to the render path:

```rust
struct TopStripGoldenVector {
    // SilkBar
    clock_h:       u8,    // hours (0-23)
    clock_m:       u8,    // minutes (0-59)
    workspace_idx: u8,    // active workspace (0..WORKSPACE_COUNT-1)
    workspace_chips: [u8; 5], // chip labels as ASCII
    bell_total:    u8,    // bell event count (0 = dim dot, >0 = gold)
    bell_redacted: u8,    // redacted count (>0 = amber dot)

    // Frame chrome (top edge only, for top strip overlap)
    frame_count:   u8,    // number of visible frames (0-3)
    frame_focused: u8,    // which frame is focused (0 = none)

    // Glass colors (from safe color pass)
    bar_fill:      u32,   // silkbar_panel_fill
    bar_text:      u32,   // silkbar_text
    chip_active:   u32,   // silkbar_chip
    chip_inactive: u32,   // tab_inactive
    frame_rim:     u32,   // frame_rim
    frame_top_bar: u32,   // frame_top_bar
    focus_surface: u32,   // focus_surface

    // Buffer dimensions
    fb_w:          u16,   // framebuffer width (pixels)
    fb_h:          u16,   // framebuffer height (pixels)
    strip_h:       u16,   // top strip height to verify (rows)
}
```

### Frozen Golden Vector (V1)

| Field | Golden Value |
|-------|-------------|
| clock_h | 10 |
| clock_m | 42 |
| workspace_idx | 0 |
| workspace_chips | ["W1","W2","W3","W4","W5"] |
| bell_total | 1 |
| bell_redacted | 0 |
| frame_count | 3 |
| frame_focused | 0 |
| bar_fill | 0x00313244 |
| bar_text | 0x00CDD6F4 |
| chip_active | 0x0089B4FA |
| chip_inactive | 0x0045475A |
| frame_rim | 0x00B4BEFE |
| frame_top_bar | 0x001E1E2E |
| focus_surface | 0x0089B4FA |
| fb_w | 1024 |
| fb_h | 768 |
| strip_h | 40 |

---

## 3. Hash / Diff Plan

### Hash Choice: xxHash64 (or simplest deterministic 64-bit hash)

- Iterate over `fb[0..fb_w * strip_h]` (row-major ARGB u32).
- Feed each pixel as 4 little-endian bytes into the hash.
- Emit a single `u64` golden hash.

### Golden Hash Storage

- Hardcoded in the proof function as `const GOLDEN_HASH: u64 = 0x...;`
- Hash is architecture-specific (little-endian, ARGB byte order).
- If the hash changes, the proof FAILS with mismatch diagnostics.

### Mismatch Diagnostics

On hash mismatch:
1. Scan row-by-row, pixel-by-pixel for first differing pixel.
2. Report: `[silk.topstrip.hash.mismatch] x=N y=N expected=0x... actual=0x...`
3. Stop after first mismatch (no full dump).
4. This pinpoints exactly which component changed.

### Hash Stability Guarantees

- Deterministic fonts (fixed glyph bitmaps, no hinting).
- Fixed color constants (no dynamic theme changes).
- No alpha compositing in the hash region.
- No clock drift (clock is frozen in the vector).
- No random/animated elements in the strip.

---

## 4. Implementation Constraints

| Constraint | Rationale |
|-----------|-----------|
| Reuse existing `fill_rect_color` / `glass_over_bg` helpers | No new render path. |
| Offscreen buffer (stack-allocated `[u32; MAX_W * STRIP_H]`) | No framebuffer ownership change. |
| No alpha/blur/shadow in hashed region | Would be non-deterministic. |
| No `render()` call — call per-component helpers directly | Isolates top strip from surface content. |
| Gate runs as a one-shot proof function | Same pattern as all other proofs. |
| sexdisplay remains sole FB writer for actual display | Offscreen buffer is test-only. |

---

## 5. Phase Ladder

### Phase 0 — THIS DOCUMENT
- Docs-only plan.

### Phase 1 — Isolated Helper Audit
- Audit `fill_rect_color`, `glass_over_bg`, font glyph drawing, clock rendering.
- Confirm each helper is deterministic given fixed inputs.
- Document any non-deterministic paths (e.g., pulse_alpha).

### Phase 2 — Offscreen Static Vector Render
- Implement `render_top_strip_golden(vector, buffer)` in sexdisplay.
- Renders: bar background, clock digits, workspace chips, Bell dot, frame rim top edge, frame lights.
- Does NOT call `render()` — only the helpers needed for the top strip.
- Marker: `[silk.topstrip.hash.vector]`

### Phase 3 — Golden Hash Gate
- Compute hash of the rendered buffer.
- Compare to hardcoded `GOLDEN_HASH`.
- If match: PASS. If mismatch: report first-pixel diff.
- Marker: `[silk.topstrip.hash.result] hash=0x... golden=0x... ok=1/0`

### Phase 4 — Mismatch Diagnostics
- On hash failure, emit first differing pixel.
- Also emit a summary of changed components (clock, bell, chips, rim, lights).
- Marker: `[silk.topstrip.hash.mismatch]`

### Phase 5 — Runtime Smoke Correlation
- Run golden hash proof in the daily driver gate.
- Correlate with QEMU visual smoke for human confirmation.
- Once hash is stable across N boots, QEMU visual inspection becomes optional.
- Marker: `[silk.topstrip.hash.proof.done]`

---

## 6. Future Markers

| Marker | Phase |
|--------|-------|
| `[silk.topstrip.hash.vector]` | 2 — Frozen vector applied, buffer rendered |
| `[silk.topstrip.hash.result]` | 3 — Hash computed, compared to golden |
| `[silk.topstrip.hash.mismatch]` | 4 — First differing pixel reported |
| `[silk.topstrip.hash.proof.done]` | 5 — Gate complete, stable across N boots |

---

## 7. STOP FIRST Boundaries

| # | Boundary | Why Blocked |
|---|----------|-------------|
| B1 | New render protocol / framebuffer ownership | sexdisplay is sole FB writer. Offscreen buffer is test-only, no protocol change. |
| B2 | Alpha/blur/shadow compositing in hash region | Non-deterministic. Must exclude or freeze. |
| B3 | Full framebuffer hash | Surface content varies per boot. Only top strip is hashable. |
| B4 | New font/glyph rendering | Must reuse existing font helpers. |
| B5 | Dynamic color themes | Must freeze colors in the golden vector. |
| B6 | Kernel/sex-pdx/global ABI edit | No ABI changes needed — proof is local to sexdisplay. |
| B7 | Renderer policy ownership change | sexdisplay remains sole owner. |
| B8 | QEMU/GUI dependency in gate | Gate runs as serial marker scan, no GUI needed. |

---

## 8. Handoff Path

```
docs/handoff/SILK_TOP_STRIP_GOLDEN_HASH_PLAN_V1.md
```

---

## 9. Commit Command

```bash
git add docs/handoff/SILK_TOP_STRIP_GOLDEN_HASH_PLAN_V1.md
git commit -m "docs(silk): top-strip golden hash plan V1"
```

---

*End of SILK_TOP_STRIP_GOLDEN_HASH_PLAN_V1.md*
