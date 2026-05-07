# QUIL_TEXT_DRAW_WIRE_V1

- date: 2026-05-07
- baseline HEAD: pending commit
- scope: Wire Quil text rendering to sexdisplay OP_TEXT_DRAW path
- previous handoff: `QUIL_MINIMAL_TEXT_SURFACE_V1.md`, `QUIL_MINIMAL_TEXT_SURFACE_BLOCKER_V1.md`
- verdict: **PASS — static verification complete; runtime pending scheduler fix**

## 1. Summary

Replaced Quil's `draw_text_lines()` fill-rect placeholder approach with actual
ASCII text sent via `OP_TEXT_DRAW` (0xFB) to sexdisplay.  The font, glyph
renderer, and opcode handler were already implemented in sexdisplay (prior
audit).  This handoff completes the Quil->sexdisplay text rendering wire-up.

## 2. Packing Format

Each `OP_TEXT_DRAW` PDX call sends up to 8 ASCII bytes:

```
pdx_call(SLOT_DISPLAY, OP_TEXT_DRAW, surface_id, packed_bytes, arg2)

arg0 = SURFACE_ID_QUIL (201)
arg1 = 8 ASCII bytes packed little-endian (byte[0] in bits 0-7, byte[7] in bits 56-63)
arg2 = byte_offset (bits 0-7) | char_count (bits 8-11) | text_color (bits 32-63)
```

**Sexdisplay unpacks:**
```rust
let sid = msg.arg0;
let packed = msg.arg1;
let byte_offset = (msg.arg2 & 0xFF) as usize;
let char_count = ((msg.arg2 >> 8) & 0xF) as usize;
let text_color = ((msg.arg2 >> 32) as u32) | 0xFF000000;
```

## 3. Line Padding Strategy

The sexdisplay 5x7 grid renderer (`surface_text_fg_at`) treats the 128-byte
text buffer as a flat character array with implicit line wrapping at
`CHARS_PER_LINE = 20` characters.  Multi-line text requires each logical
line to be padded to exactly 20 characters (trailing spaces) so each
logical line occupies its own raster row.

Quil splits its buffer on `\n`, pads each logical line via `pad_text_line()`,
and sends the resulting flat buffer in 8-byte chunks.

## 4. Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/quil/src/main.rs` | Import OP_TEXT_DRAW/CLEAR; rewrite `draw_text_lines()` to send text via 0xFB; add `pad_text_line()` helper | +60/-45 |
| `servers/sexdisplay/src/main.rs` | Add `[sexdisplay.text.draw]` diagnostic marker in OP_TEXT_DRAW handler | +4 |
| `docs/handoff/QUIL_TEXT_DRAW_WIRE_V1.md` | This handoff | new |

## 5. Files NOT Changed

| File | Reason |
|------|--------|
| `crates/sex-pdx/src/lib.rs` | OP_TEXT_DRAW/CLEAR already defined (prior audit) |
| `servers/sexdisplay/src/main.rs` (font/renderer) | Already implemented (prior audit) |
| `kernel/src/` | No kernel edits for text rendering |
| Any other server | Text rendering is Quil->sexdisplay only |

## 6. Build Result

```
./scripts/entrypoint_build.sh -> PASS (exit 0)
```

- Quil: 21 pre-existing warnings only (no new warnings)
- sexdisplay: 0 warnings
- Full ISO: 1714 sectors

## 7. No-Go Boundaries Preserved

- [x] No framebuffer access from Quil (PDX-only)
- [x] No kernel edits
- [x] No sex-pdx ABI break (additive constants only)
- [x] sexdisplay sole framebuffer writer preserved
- [x] MPK/PKU/PKEY isolation intact
- [x] No shared-memory redesign
- [x] No POSIX/Linux/std/libc/threads
- [x] No broad refactor

## 8. Static Verification (all PASS)

- [x] Packing format matches sexdisplay unpacking bit-for-bit
- [x] OP_TEXT_CLEAR called before new text
- [x] Line padding correct (20 chars per raster row)
- [x] 8-byte chunking with correct byte_offset advancement
- [x] Text color (0x00E0F0FF) packed in arg2 bits 32-63
- [x] surface_text_fg_at() renders from text_buf via FONT_ASCII_5X7
- [x] Ownership authorization (owner_pd check) in both handlers

## 9. Runtime Gate Status

SPAWN_GATE=PASS (all 12 PDs enqueued).  FAULT_GATE=PASS (zero panics/faults).
SCHED_GATE shows only PD1 `task.running` (pre-existing scheduler tick cadence
issue in QEMU test environment — CLOCK_GATE was FAIL before this change).

Quil PD9 is enqueued by the scheduler but the QEMU test window expires before
it gets scheduled.  The OP_TEXT_DRAW path is verified through static analysis
-- runtime proof requires either a longer QEMU window or real hardware boot.

## 10. Diagnostic Marker

sexdisplay OP_TEXT_DRAW handler now emits:

```
[sexdisplay.text.draw] sid=<surface_id> len=<text_len> color=<ARGB>
```

This fires on the first text chunk (len <= 32 guard) and confirms the text
buffer was populated.

## 11. Next Steps

1. **Scheduler fix**: Investigate why only PD1 runs in QEMU.  LAPIC timer
   calibration was added but uses PIT channel 2 which may behave differently
   in QEMU.  If timer isn't firing periodic ticks, scheduler stalls.

2. **Quil serial output**: Quil's `serial_println!` via `pdx_call(0,69,...)`
   produces no output.  Check kernel raw_print capability grant for Quil PD9.

3. **Real hardware text proof**: Boot on real hardware with serial capture
   to verify `[sexdisplay.text.draw]` and visible text glyphs on screen.

## 12. Recurring Issue: Quil Serial Output Silent

Quil's `serial_println!` calls produce no output in QEMU serial log.
Possible causes:
- Kernel `raw_print` syscall (0, 69) requires capability grant missing for PD9
- PDX slot 0 not configured for Quil in kernel init.rs

Workaround: Use sexdisplay-side diagnostic markers (`[sexdisplay.text.draw]`)
to verify PDX message delivery.  Sexdisplay serial output works correctly.
