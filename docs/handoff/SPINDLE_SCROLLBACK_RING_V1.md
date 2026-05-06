# SPINDLE_SCROLLBACK_RING_V1

**Date:** 2026-05-06
**Status:** Scrollback ring proven — 1024-line ring buffer, line clamping, scroll offset
**Contract:** docs/handoff/SPINDLE_APP_CONTRACT_V1.md
**Previous:** SPINDLE_KEYBOARD_INPUT_LINE_V1
**Next:** SPINDLE_NATIVE_COMMAND_DISPATCH_V1

---

## Summary

Added a bounded scrollback ring buffer to Spindle:
- Fixed-size ring: 1024 lines x 80 bytes = 80 KiB (BSS, no allocation)
- Enter pushes command line into scrollback
- Boot header lines (4 lines) pre-loaded
- 18 visible output rows (rows 5-22, 80-column CP437 grid)
- Scroll offset for PageUp/PageDown navigation
- Line clamping at 80 bytes — longer lines truncated safely
- Ring wrap on overflow — oldest lines overwritten
- Proof gate extended with 3 new scrollback stages

---

## Files Changed

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +115 lines — Scrollback struct, render_scrollback, proof stages 7-9 |
| `docs/handoff/SPINDLE_SCROLLBACK_RING_V1.md` | NEW |

## Files NOT Changed

| File | Reason |
|------|--------|
| `kernel/src/init.rs` | STOP FIRST (Spindle not kernel-spawned) |
| `crates/sex-pdx/` | No ABI changes |
| `servers/silk-shell/` | No input routing changes |
| `servers/sexdisplay/` | No display changes |

---

## Scrollback Ring Design

```
┌─────────────────────────────────────────┐
│ ring: [[u8; 80]; 1024]   ← 80 KiB BSS   │
│ write_pos: wraps 0..1023                │
│ total_lines: monotonic u32, never wraps │
│ scroll_offset: 0 = show latest          │
└─────────────────────────────────────────┘

Display mapping:
  total_lines=1050, scroll_offset=5, VISIBLE_ROWS=18
  → newest visible = 1050 - 1 - 5 = 1044
  → oldest visible = 1044 - 17 = 1027
  → ring_idx = line % 1024 for each visible line
```

### Bounded Dimensions

| Parameter | Value | Storage |
|-----------|-------|---------|
| Max scrollback lines | 1024 | `[[u8; 80]; 1024]` |
| Max chars per line | 80 (COLS) | `const MAX_LINE_BYTES` |
| Visible output rows | 18 (rows 5-22) | `const VISIBLE_ROWS` |
| Output row start | 5 | `const OUTPUT_ROW_START` |
| Scroll offset max | u32::MAX | wraps safely |
| Total ring size | 80 KiB | BSS, zero allocation |

---

## API

```rust
struct Scrollback {
    ring: [[u8; 80]; 1024],
    write_pos: usize,     // wraps 0..1023
    total_lines: u32,     // monotonic, saturating at u32::MAX
    scroll_offset: u32,   // user scroll position
}

impl Scrollback {
    fn push(&mut self, line: &[u8])            // push one line, clamp to 80 bytes
    fn get(&self, ring_idx: usize) -> &[u8]    // read line from ring (NUL-terminated)
}

unsafe fn render_scrollback(fb, sb)            // draw visible rows 5-22
```

### Line Clamping

Lines longer than 80 bytes are silently truncated. Zero-fill pads short lines. This prevents buffer overflows and guarantees fixed-width rendering.

### Ring Overflow

When `write_pos` wraps past 1024, the oldest entry is overwritten. `total_lines` continues to increase past 1024. The render function correctly calculates ring indices using `line % 1024`.

---

## Proof Gate (Extended)

### New Stages (7-9)

| Stage | Operation | Assertion | Marker |
|-------|-----------|-----------|--------|
| 7 | Push 2048 lines (2x capacity) | Ring wraps, total > 1024, no panic | `[spindle.scrollback.overflow]` |
| 8 | Push 200-byte line | Clamped to 80 bytes | `[spindle.scrollback.clamp]` |
| 9 | Set scroll_offset=10, render, reset | No crash, correct index math | `[spindle.scrollback.render]` |

### Existing Stages (1-6)

Preserved from SPINDLE_KEYBOARD_INPUT_LINE_V1 — append, backspace, overflow, non-printable, enter, empty-backspace.

### Updated Stage 5 (Enter)

Now pushes the command line into scrollback before clearing:
```
sb.push(line.as_bytes());
line.clear();
render_scrollback(fb, sb);
```

---

## Surface Layout (Updated)

```
Row 0:  ┌──────────────────────────────────────────┐
Row 1:  │ Spindle                              ← accent
Row 2:  │ SexOS native command console          ← FG
Row 3:  │ Type help for commands.               ← FG
Row 4:  │──────────────────────────────────────← separator
Row 5:  │ Spindle -- SexOS native command console│ ← scrollback
Row 6:  │                                          │
Row 7:  │ Type help for commands. V1.0.0-pre       │
Row 8:  │                                          │
Row 9:  │ test                                     │ ← entered command
  ...   │   (rows 10-22: remaining scrollback)      │
Row 23: │ sex> _                                ← prompt + cursor
        └──────────────────────────────────────────┘
```

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| cargo check | PASS (4 warnings) |
| entrypoint_build.sh | PASS |
| master_runtime_gate | **GREEN_MASTER** (6/6) |
| Faults | **0** |

---

## Scroll Controls (Future)

Scroll offset state is functional (`sb.scroll_offset`) but no keyboard shortcuts are wired (PageUp/PageDown requires HID delivery, which is blocked on kernel spawn). When HID delivery works:

```
PageUp   → scroll_offset = min(scroll_offset + VISIBLE_ROWS, total_lines - VISIBLE_ROWS)
PageDown → scroll_offset = scroll_offset.saturating_sub(VISIBLE_ROWS)
Home     → scroll_offset = total_lines - VISIBLE_ROWS  (latest)
End      → scroll_offset = 0  (oldest)
```

---

## Next Prompt

```
SPINDLE_NATIVE_COMMAND_DISPATCH_V1
```

Adds: compile-time command table, help/ver/echo/clear commands, command dispatch on Enter, output to scrollback.

---

## Contract Boundaries Preserved

- **No kernel edits** — synthetic proof, no kernel spawn
- **No sex-pdx ABI edits** — no new slots
- **No silk-shell changes** — no input routing
- **sexdisplay sole FB writer** — Spindle writes within bounded window region
- **FB bounds checks** — WindowBuffer validates all draw calls
- **No heap growth** — 80 KiB scrollback ring is static BSS
- **No unbounded Vec/String** — fixed arrays only
- **Line clamping** — all copy operations bounded to MAX_LINE_BYTES
