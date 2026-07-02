# INPUT_CLICK_FOCUS_PROOF_V1

- date: 2026-05-07
- baseline HEAD: pending commit
- scope: Add proof markers verifying the pointer click→focus→drag chain
- verdict: **PASS — static verification complete; runtime pending scheduler**

## 1. Summary

Added 8 proof markers across sexinput and silk-shell to create a complete,
greppable evidence chain for the pointer button press/release → click →
focus change → drag start/end path.

All markers are budgeted (no log storms), additive (no behavior changes),
and preserve the existing HID/PDX ABI.

## 2. Proof Marker Chain

```
sexinput (PD 4)                           silk-shell (PD 3)
─────────────────                         ──────────────────
[sexinput.drag_proof.start]   ──EV_ABS──→
[sexinput.drag_proof.down]    ──EV_BTN──→  [silk-shell.pointer.recv]
                              btn=1,val=1  [silk-shell.click.down]
                                           [shell.click.real.target]
                                           [silk-shell.focus.change]
                                           [shell.interact.drag.begin]
[sexinput.drag_proof.move]    ──EV_REL──→  [silk-shell.pointer.recv]
                              dx,dy        [shell.interact.drag.move]
[sexinput.drag_proof.up]      ──EV_BTN──→  [silk-shell.pointer.recv]
                              btn=1,val=0  [silk-shell.click.up]
                                           [shell.interact.drag.end]
[sexinput.drag_proof.done]
```

## 3. New Markers Added

### sexinput (`servers/sexinput/src/main.rs`)

| Marker | Location | Condition |
|--------|----------|-----------|
| `[sexinput.pointer.button.down]` | Forward path (line ~251) | cls==EV_BTN && pressed |
| `[sexinput.pointer.button.up]` | Forward path (line ~253) | cls==EV_BTN && !pressed |

Existing markers retained: `[sexinput.pointer.recv]`, `[sexinput.pointer.send]`,
`[sexinput.drag_proof.*]`.

### silk-shell (`servers/silk-shell/src/main.rs`)

| Marker | Location | Condition |
|--------|----------|-----------|
| `[silk-shell.pointer.recv]` | HID event handler | class=EV_REL/EV_ABS/EV_BTN |
| `[silk-shell.click.down]` | BTN handler (line ~11785) | button==1 && pressed && (Idle or PanelActive) |
| `[silk-shell.click.up]` | BTN handler (line ~11804) | button==1 && !pressed && ClickPending |
| `[silk-shell.focus.change]` | try_set_focus (line ~8888) | focus cleared or set to new sid |

Existing markers retained: `[shell.click.real.target]`,
`[shell.interact.drag.begin]`, `[shell.interact.drag.end]`,
`[shell.interaction.transition]`, `[shell.interact.drag.move]`,
`[shell.click_focus.down]`.

## 4. Files Changed

| File | Lines | Changes |
|------|-------|---------|
| `servers/sexinput/src/main.rs` | +6 | button.down/button.up markers in forward path |
| `servers/silk-shell/src/main.rs` | +8 | pointer.recv, click.down, click.up, focus.change markers |
| `docs/handoff/INPUT_CLICK_FOCUS_PROOF_V1.md` | new | this handoff |

## 5. Files NOT Changed

| File | Reason |
|------|--------|
| `kernel/src/` | No kernel/IRQ/capability edits needed |
| `crates/sex-pdx/src/lib.rs` | No ABI changes |
| `servers/sexusb/src/main.rs` | USB→sexinput path unchanged |
| `servers/sexdisplay/src/main.rs` | No renderer changes |

## 6. Build Result

```
./scripts/entrypoint_build.sh → PASS (exit 0)
```

- sexinput: 0 new warnings
- silk-shell: 0 new warnings (pre-existing deprecation warnings only)
- Full ISO: 1714 sectors

## 7. No-Go Boundaries Preserved

- [x] No kernel/IRQ/capability edits
- [x] No sex-pdx ABI changes
- [x] No HID/PDX protocol redesign
- [x] No input policy moved into sexdisplay
- [x] No gestures implemented (stick to click/focus/drag only)
- [x] No framebuffer access from sexinput
- [x] silk-shell owns click/focus/drag policy unchanged
- [x] No shared-memory redesign

## 8. Synthetic Proof Path

The existing `SYNTHETIC_INPUT_PROOFS` path in sexinput generates the full
button down → move → button up sequence at tick % 120 == 0:

```
[sexinput.drag_proof.start]   → EV_ABS(200,200)   [move pointer]
[sexinput.drag_proof.down]    → EV_BTN(1,1)       [left button press]
[sexinput.drag_proof.move]    → EV_REL(6,4)        [drag movement]
[sexinput.drag_proof.up]      → EV_BTN(1,0)        [left button release]
[sexinput.drag_proof.done]                          [one-shot gate prevents replay]
```

These events flow to silk-shell via SLOT_SHELL→OP_HID_EVENT and trigger the
full click/focus/drag state machine.

## 9. Runtime Gate Status

SPAWN_GATE=PASS (12 PDs enqueued).  FAULT_GATE=PASS (zero panics).  The
proof markers do not appear in QEMU because the scheduler only reaches PD1
(sexdisplay) within the 30s test window (pre-existing CLOCK_GATE=FAIL).
sexinput (PD4) and silk-shell (PD3) event loops are not reached.

Markers are verified through static analysis.  Runtime proof requires a
scheduler tick cadence fix or real hardware boot.

## 10. Extending This Proof

To add new interaction proofs (e.g., right-click, double-click, gesture):
1. Add new markers in the same handler locations
2. Follow the existing budgeted-marker pattern (static mut counter, decrement)
3. Do not add new PDX opcodes unless STOP FIRST approved
4. Use the synthetic proof path in sexinput for deterministic QEMU testing
