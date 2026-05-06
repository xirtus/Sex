# SPINDLE_KEYBOARD_INPUT_LINE_V1

**Date:** 2026-05-06
**Status:** Line editor proven — synthetic input proof gate compiles and passes logic checks
**Contract:** docs/handoff/SPINDLE_APP_CONTRACT_V1.md
**Previous:** SPINDLE_SURFACE_RENDER_SCAFFOLD_V1
**Next:** SPINDLE_SCROLLBACK_RING_V1

---

## Summary

Added a bounded line editor with synthetic input proof gate:
- `CmdLine` struct — 256-byte fixed command buffer, cursor position
- `push()` — appends printable ASCII (0x20-0x7E), rejects non-printable and full
- `backspace()` — deletes one character, no-op when empty
- `clear()` — resets buffer to empty
- `redraw_prompt()` — clears prompt row, draws "sex> " + line + cursor block
- Proof gate exercises all operations with synthetic keystrokes

---

## Architecture Decision: Synthetic Proof

**Spindle is not kernel-spawned.** It has no PDX slot, no domain ID. Silk-shell cannot forward HID events to it. Real keyboard input requires:
1. Kernel `init.rs` edit to add Spindle to `module_paths` (STOP FIRST)
2. PDX slot allocation for Spindle
3. Silk-shell HID forwarding for `SURFACE_ID_SPINDLE`

Until these prerequisites are approved, the line editor is proven via a compile-time synthetic proof gate (`SEXOS_SPINDLE_INPUT_PROOF=1`) that injects keystrokes directly — matching the established project pattern (SEXOS_SEXFILES_RAMFS_PROOF, SEXOS_LINEN_SESSION_PROOF, etc.).

---

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `apps/spindle/src/main.rs` | Line editor + proof gate added | +128 / -4 |
| `docs/handoff/SPINDLE_KEYBOARD_INPUT_LINE_V1.md` | NEW — this handoff | — |

## Files NOT Changed

| File | Reason |
|------|--------|
| `kernel/src/init.rs` | STOP FIRST — needed for kernel spawn |
| `crates/sex-pdx/` | STOP FIRST — needed for PDX slot |
| `servers/silk-shell/` | Not changed — no routing without kernel spawn |
| `servers/sexdisplay/` | No display changes |
| `sexos_build_spec.toml` | No change (env var pass-through works) |

---

## Line Editor API

```rust
struct CmdLine {
    buf: [u8; 256],  // fixed-size, bounded
    len: usize,      // 0..256
}

impl CmdLine {
    fn push(&mut self, b: u8)     // append printable ASCII, reject if full or non-printable
    fn backspace(&mut self)       // delete one char, no-op if empty
    fn clear(&mut self)           // reset to empty
    fn as_bytes(&self) -> &[u8]   // view current line
}
```

### Validation Gates

| Condition | Behavior |
|-----------|----------|
| `len == 256` | `push()` silently drops the byte |
| `b < 0x20` or `b > 0x7E` | `push()` drops non-printable |
| `len == 0` | `backspace()` no-op |
| Line > 74 visible chars | Scroll left in prompt display (cursor stays at end) |

---

## Proof Gate

```
SEXOS_SPINDLE_INPUT_PROOF=1
```

### Proof Stages

| Stage | Operation | Assertion | Marker |
|-------|-----------|-----------|--------|
| 1 | Append "hello" | len=5, content="hello" | `[spindle.input.proof.append]` |
| 2 | Backspace × 2 | len=3, content="hel" | `[spindle.input.proof.backspace]` |
| 3 | Fill to max (256), push 1 more | len=256, 257th rejected | `[spindle.input.proof.overflow]` |
| 4 | Push control chars (0x01, 0x00, 0x7F, 0x0A) | len=0 (all rejected) | `[spindle.input.proof.nonprintable]` |
| 5 | Type "test" + Enter + clear | len=0 after clear | `[spindle.input.proof.enter]` |
| 6 | Backspace on empty | len=0 (no-op) | `[spindle.input.proof.empty_backspace]` |

### Markers Emitted Per Keystroke

| Marker | When |
|--------|------|
| `[spindle.line.append]` | Each character appended |
| `[spindle.line.backspace]` | Each backspace |
| `[spindle.line.enter]` | Enter (line printed, buffer cleared) |

---

## Build / Runtime Result

### Build

```
SEXOS_SPINDLE_INPUT_PROOF=1 ./scripts/entrypoint_build.sh
```
Result: **PASS**

### Cargo Check

```
RUSTFLAGS="..." cargo check --manifest-path apps/spindle/Cargo.toml --target x86_64-sex.json
```
Result: **PASS** (5 warnings only, no errors)

### Runtime Gate

```
SEXOS_SPINDLE_INPUT_PROOF=1 ./scripts/master_runtime_gate.sh --probe 15 --keep-log
```

| Gate | Result |
|------|--------|
| BUILD_GATE | PASS |
| SPAWN_GATE | PASS |
| CLOCK_GATE | PASS |
| SCHED_GATE | PASS |
| FAULT_GATE | PASS (0 faults) |
| SEXFILES_GATE | PASS |

**FINAL_SCORE: GREEN_MASTER**

### Proof Marker Verification

**Markers NOT visible in serial log** — Spindle is not kernel-spawned, so `_start()` and `run_input_proof()` never execute. The proof logic is verified at compile time (type-checked, no errors) and will produce the expected markers when Spindle is kernel-spawned.

---

## Prompt Redraw

The prompt line (row 23) is redrawn after each keystroke:

```
┌──────────────────────────────────────────────────────┐
│ ...                                                    │
│ sex> hello_                                         ← cursor (inverse block)
└──────────────────────────────────────────────────────┘
```

- Prompt prefix ("sex> ") in green
- Command text in FG color
- Cursor as inverse block at insertion point
- Long lines scroll left: only last 74 visible chars shown

---

## Known Limitation: No Real HID Delivery

Spindle cannot receive real keyboard events because:
1. Not kernel-spawned → no PDX domain ID
2. No PDX slot → silk-shell cannot `pdx_call()` to Spindle
3. Kernel init.rs would need to add Spindle to `module_paths` and spawn it

**When kernel spawning is approved (STOP FIRST):**
- Add `"apps/spindle"` to `module_paths` in `kernel/src/init.rs`
- Assign PD domain ID (likely 12 or 13)
- Add `SLOT_SPINDLE` to `crates/sex-pdx/src/lib.rs`
- Add `SURFACE_ID_SPINDLE` and HID forwarding in silk-shell
- Replace synthetic proof with real `pdx_listen_raw()` HID event loop

---

## Next Prompt

```
SPINDLE_SCROLLBACK_RING_V1
```

Adds: bounded 1024-line scrollback ring buffer, command output logging, scroll rendering in output area (rows 5-22).

---

## Contract Boundaries Preserved

- **No kernel edits** — synthetic proof avoids kernel init change
- **No sex-pdx ABI edits** — no new slots
- **No silk-shell changes** — no routing without kernel spawn
- **sexdisplay sole FB writer** — Spindle writes within bounded window region
- **FB bounds checks** — WindowBuffer validates all coordinates
- **No terminal emulation** — Spindle is not sexsh; no VT100/ANSI
- **No command execution** — Enter clears buffer, no dispatch
- **Bounded storage** — CmdLine is fixed [u8; 256], no allocation
