# SPINDLE_INPUT_RENDER_CORE_V1

**Date:** 2026-05-07
**Status:** Core proven — scrollback ring, text rendering, cursor, proof markers
**Previous:** SPINDLE_TERMINAL_FINISH_PLAN_V1
**Next:** (integration proof)

---

## Summary

Enhanced the Spindle terminal core (YarnSession inside silk-shell) with:
- Bounded scrollback ring (1024 lines × 80 bytes = 80 KiB BSS)
- Command-line text rendering via sexdisplay's 5×7 ASCII font (0xFA/0xFB)
- Block cursor via fill rect (0xEF, rect_index=7)
- Real-time render-on-keystroke (cursor follows input)
- Proof markers: input.recv, line.edit, render.submit, scrollback.push

---

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/silk-shell/src/main.rs` | Scrollback ring + text/cursor render + proof markers | ~116 |
| `docs/handoff/SPINDLE_INPUT_RENDER_CORE_V1.md` | NEW — this handoff | — |

## Files NOT Changed

| File | Reason |
|------|--------|
| `crates/sex-pdx/` | No ABI changes needed (SLOT_SPINDLE=14 pre-existing) |
| `servers/sexdisplay/` | No display changes (0xFA/0xFB/0xEF pre-existing) |
| `apps/spindle/` | Independent PD (receives forwarded HID, does serial output) |
| `kernel/` | No kernel edits |

---

## Architecture

```
sexinput → silk-shell (focus gate: FOCUSED_SURFACE_ID == SURFACE_ID_SPINDLE)
               │
               ├── pdx_call(SLOT_SPINDLE, OP_HID_EVENT) → spindle PD (serial only)
               │
               ├── YARN.cmd_buf ← scancode→char (local editing)
               ├── spindle_dispatch() ← on Enter (local dispatch)
               ├── yarn_append_output() → sb_ring + output_lines
               │
               └── spindle_render() → sexdisplay
                       ├── 0xEF fill rects: header bar + 7 output bands
                       ├── 0xFA text clear + 0xFB text draw: cmdline glyphs
                       └── 0xEF fill rect: block cursor (rect_index=7)
```

---

## Scrollback Ring

| Parameter | Value | Storage |
|-----------|-------|---------|
| Max lines | 1024 | `[[u8; 80]; 1024]` (80 KiB BSS) |
| Line width | 80 bytes | `SPINDLE_SB_LINE_CAP` |
| Write pos | wraps 0..1023 | `sb_write: usize` |
| Total lines | monotonic u32 | `sb_total: u32` |
| Scroll offset | 0 = latest | `sb_offset: u32` |

Pushed in `yarn_append_output()`. Cleared in `yarn_cmd_clear()`.

---

## Text/Cursor Rendering

### Text (0xFB OP_TEXT_DRAW)
- 8 bytes per PDX call, packed LE into arg1
- Byte offset + char count + color in arg2
- Text appears at surface-relative (8, 24) in 5×7 ASCII font
- Command line: "sex> " + current buffer (40 chars max, 2 text lines)

### Cursor (0xEF OP_SURFACE_FILL_RECT)
- Block cursor fill rect at rect_index=7
- Position: below band area, x tracks prompt + input length
- Color: Catppuccin Rosewater (0xFFF5E0DC)

### Render Triggers
| Event | Calls |
|-------|-------|
| Backspace | `spindle_render()` |
| Escape (clear) | `spindle_render()` |
| Printable char | `spindle_render()` |
| Enter | `spindle_dispatch()` → `spindle_render()` |

---

## Proof Markers

| Marker | Source | When |
|--------|--------|------|
| `[spindle.input.recv]` | silk-shell keyboard handler | Every keystroke to Spindle surface |
| `[spindle.line.edit]` | silk-shell keyboard handler | On backspace/escape/push |
| `[spindle.render.submit]` | `spindle_render_cmdline()` | After text+cursor rendered to sexdisplay |
| `[spindle.scrollback.push]` | `yarn_append_output()` | Each output line appended to scrollback ring |
| `[spindle.render.done]` | `spindle_render()` | After full render cycle complete |

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| cargo check | PASS (0 new errors, 0 new warnings) |
| Non-goals preserved | All (no kernel/ABI/sexdisplay edits) |
| FB bounds | sexdisplay validates all 0xEF/0xFB calls |

### Build Command
```sh
RUSTFLAGS="-C target-cpu=generic" cargo +nightly build -p silk-shell \
  --target x86_64-sex.json -Zunstable-options \
  -Zbuild-std=core,alloc,compiler_builtins --release
```

Alternatively: `./scripts/entrypoint_build.sh`

### Runtime Success Signal
- Spindle surface opens (Scroll Lock toggle or shell action)
- Keys typed to Spindle surface update cursor position in real time
- Enter dispatches command, output appears as colored bands
- Serial log shows `[spindle.input.recv]`, `[spindle.line.edit]`, `[spindle.render.submit]`, `[spindle.scrollback.push]`

---

## Contract Boundaries Preserved

- **No Linux/POSIX assumptions** — pure no_std, no PTY/TTY
- **No kernel edits** (0 changes)
- **No sex-pdx ABI edits** (0 changes)
- **sexdisplay sole FB writer** — Spindle renders via 0xEF/0xFA/0xFB PDX calls
- **No shared-memory redesign** — all rendering through PDX registers
- **SilkBar/top strip/clock preserved** — sexdisplay renders bar at y<50, Spindle surface at y>=200
- **FB bounds checks preserved** — sexdisplay validates all fill rects and text against surface bounds
- **Bounded storage** — scrollback ring is fixed 80 KiB BSS, no heap growth
