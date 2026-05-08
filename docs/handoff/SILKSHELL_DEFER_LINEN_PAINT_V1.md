# SILKSHELL_DEFER_LINEN_PAINT_V1

**Date:** 2026-05-07
**Status:** IMPLEMENTED — sexinput→silk-shell route fully proven

## Summary

Silk-shell's boot-time `linen_paint_surface()` called `linen_sync_reply()`
which did a blocking `pdx_listen_raw(0)` loop, silently dropping all
non-Linen messages (including OP_HID_EVENT from sexinput).

**Fix:** defer `linen_paint_surface()` to first main-loop iteration.
During synthetic input proofs, skip it entirely (Linen fetch is cosmetic).
`linen_sync_reply` now calls `pdx_reply` on non-Linen messages to unblock
senders instead of dropping them silently.

## Proof Marker Chain

```
[silk-shell.ready]
[silk-shell.linen.paint.skip] reason=synthetic_gate_active
[sexusb.synthetic_slot2.begin]
[sexusb.synthetic_slot2.report] ... ×7
  → [sexinput.pointer.recv]
  → [sexinput.pointer.forward.reason=motion]
  → [sexinput.pointer.send]
  → [sexinput.hid.emit.rel]
  → [silk-shell.pointer.recv] class=3 a0=200 a1=200    ← EV_ABS
  → [silk-shell.cursor.update] x=200 y=200              ← cursor
  → [silk-shell.pointer.recv] class=EV_BTN btn=1        ← button
  → [silk-shell.click.down] btn=1 x=200 y=200           ← click
  → [silk-shell.pointer.recv] class=2 a0=6 a1=4         ← EV_REL
  → [silk-shell.cursor.update] x=646 y=364              ← moved
  → [silk-shell.pointer.recv] class=EV_BTN pressed=false ← release
  → [silk-shell.click.up] btn=1 x=646 y=364             ← click up
[sexusb.synthetic_slot2.done]
```

## Runtime Counts

| Marker | Count |
|--------|-------|
| silk-shell.ready | 1 |
| silk-shell.pointer.recv | 12 |
| silk-shell.cursor.update | 8 |
| silk-shell.click.down/up | 4 |
| sexinput.pointer.send | 7 |
| sexinput.pointer.drop | 0 |
| #PF/#GP/panic | 0 |

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +12 lines: defer linen paint + pdx_reply fix + synthetic skip |
| `servers/sexusb/src/main.rs` | +2 lines: yield delay before synthetic gate |
| `docs/handoff/SILKSHELL_DEFER_LINEN_PAINT_V1.md` | Created |

## USB 100% Progress

| # | Item | Status |
|---|------|--------|
| 1-8 | C1 through C2C/C2E | ✅ |
| 9 | sexinput→silk-shell route | ✅ **PROVEN** |
| 10 | Pointer → cursor.move | ✅ |
| 11 | Button → click.focus | ✅ |
| 12 | Real hardware tablet data | ⬜ Last remaining gap |

---

*End of SILKSHELL_DEFER_LINEN_PAINT_V1.md*
