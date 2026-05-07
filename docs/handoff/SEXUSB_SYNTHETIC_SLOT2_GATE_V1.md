# SEXUSB_SYNTHETIC_SLOT2_GATE_V1

**Date:** 2026-05-07
**Status:** IMPLEMENTED — build + runtime PASS

## Summary

Added a compile-time `SEXUSB_SYNTHETIC_SLOT2` gate that injects 7 slot2-shaped
pointer reports through the existing normalizer → OP_HID_EVENT pipeline.
Zero poll-loop overhead.  Dead code when gate unset.

Proves: sexusb → sexinput normalizer → OP_HID_EVENT works for slot2 data
without touching the cooperative scheduler hot path.

## 1. Gate Design

```rust
const SEXUSB_SYNTHETIC_SLOT2: bool =
    option_env!("SEXUSB_SYNTHETIC_SLOT2").is_some();
```

- Default (unset): dead code, zero overhead
- `SEXUSB_SYNTHETIC_SLOT2=1` at build: injects 7 reports, falls through to poll loop

## 2. Report Sequence

| Frame | buttons | dx | dy | Purpose |
|-------|---------|----|----|---------|
| 0-4 | 0 | 1 | 1 | Move pointer toward target |
| 5 | 1 | 0 | 0 | Button down (click) |
| 6 | 0 | 0 | 0 | Button up (release) |

All sent via existing `send_synthetic_mouse_frame()` → `OP_USB_MOUSE_REPORT`
→ sexinput normalizer (`normalize_pointer_report_v1`) → `OP_HID_EVENT` →
silk-shell.

## 3. Proof Marker Chain (Observed)

```
[sexusb.synthetic_slot2.begin]
[sexusb.synthetic_slot2.report] n=0 buttons=0 dx=1 dy=1
  → [sexinput.pointer.recv] class=0
  → [sexinput.pointer.forward.reason=motion]
  → [sexinput.pointer.send] class=2 a0=1 a1=1
  → [sexinput.hid.emit.rel] n=0 dx=1 dy=1
... (reports 1-4 same pattern) ...
[sexusb.synthetic_slot2.report] n=5 buttons=1 dx=0 dy=0    ← button down
[sexusb.synthetic_slot2.report] n=6 buttons=0 dx=0 dy=0    ← button up
[sexusb.synthetic_slot2.done]
```

Shell markers (`silk-shell.pointer.recv`, `silk-shell.click`) not observed
due to pre-existing cooperative scheduler not reaching silk-shell within
the 20s test window — same CLOCK_GATE=FAIL as INPUT_CLICK_FOCUS_PROOF_V1.
Pipeline leg sexusb→sexinput is proven; shell leg proven by synthetic drag
proof (INPUT_PHASE_CLOSEOUT_V1).

## 4. Fault Scan

| Type | Count |
|------|-------|
| #PF | 0 |
| #GP | 0 |
| panic | 0 |
| page fault | 0 |

## 5. Build Result

```
Default:    ./scripts/entrypoint_build.sh          → PASS, 1766 sectors
Gate on:    SEXUSB_SYNTHETIC_SLOT2=1 ./scripts/... → PASS, 1766 sectors
```

## 6. Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | +30 lines: SEXUSB_SYNTHETIC_SLOT2 gate + 7-frame sequence |
| `docs/handoff/SEXUSB_SYNTHETIC_SLOT2_GATE_V1.md` | Created |

## 7. Invariants

| Check | Status |
|-------|--------|
| C1 `slot2.poll.start` present | ✅ |
| Poll loop unchanged | ✅ |
| No scheduler starvation (gate outside hot loop) | ✅ |
| No kernel/sex-pdx/sexinput/shell/display edits | ✅ |
| Default build zero overhead | ✅ |

## 8. Next Phase

**SEXUSB_BUDGETED_POLL_YIELD_V1 (Option B)** — Add budgeted yield to poll
loop so C2B-C2E real USB work can proceed without scheduler starvation.

---

*End of SEXUSB_SYNTHETIC_SLOT2_GATE_V1.md*
