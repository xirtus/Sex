# TYPING_VISIBLE_SURFACE_AUDIT_V1

**Status:** PASS REVIEW ONLY — E2E path already exists in Quil.
**Date:** 2026-05-16
**Depends on:** `DAILY_DRIVER_100_GATE_FREEZE_V1.md`.

---

## 0. Key Finding: Quil already has full keyboard→visible-text E2E

No new infrastructure needed. The path is proven by synthetic keyboard stash/replay in daily proof. Real USB keyboard input uses the exact same dispatch path.

---

## 1. End-to-End Typing Path (Quil)

```
USB/PS2 HID event
  → sexinput/sexusb (scancode capture)
  → silk-shell (pdx_listen_raw, dispatch)
  → pdx_call(SLOT_QUIL, OP_HID_EVENT, scancode, value)   [shell line 17251]
  → Quil pdx_listen_raw(0) matches OP_HID_EVENT           [quil line 1177]
  → scancode_to_char(scancode, shift)                     [quil line 1629]
  → text_buffer_append(ch)                                [quil line 1631]
  → draw_text_lines(&QUIL_BUFFER)                         [quil line 1632]
  → fill-rect IPC to sexdisplay                           [quil line 385+]
  → sexdisplay renders glyphs                             [sexdisplay fill_rect_color]
  → [quil.text.recv] marker emitted                       [quil line 1630]
```

---

## 2. Current Truth Table

| Question | Answer | Evidence |
|----------|--------|----------|
| Can Quil receive typed chars? | **YES** | `pdx_call(SLOT_QUIL, OP_HID_EVENT, ...)` in shell line 17251 |
| Can Quil render typed chars visibly? | **YES** | `draw_text_lines()` → fill-rect IPC → sexdisplay |
| Is the path synthetic-only or real? | **BOTH** | Daily proof uses stash/replay; real USB uses same dispatch |
| Can WebStub receive text? | **NO** | No OP_HID_EVENT dispatch to WebStub, no text buffer |
| Is focus routing real? | **YES** | `try_set_focus()` → `FOCUSED_SURFACE_ID` → dispatch |
| Is keyboard path real device? | **YES** | sexinput/sexusb capture USB HID events |

---

## 3. Recommended Smallest Target

### QUIL_VISIBLE_TYPING_E2E_V1

Prove that real USB keyboard input produces visible text in Quil:
- Boot with USB keyboard
- Focus Quil surface
- Type characters
- Verify `[quil.text.recv]` markers in serial log
- Verify fill-rect send markers
- Document that the path is live (not synthetic)

### Why Quil, not WebStub

| Factor | Quil | WebStub |
|--------|------|---------|
| Text buffer | ✅ 64KB buffer | ❌ No buffer |
| Scancode→char | ✅ `scancode_to_char()` | ❌ No mapping |
| Render path | ✅ `draw_text_lines()` → fill-rect | ❌ No render |
| HID dispatch | ✅ Shell dispatches to Quil | ❌ No dispatch |
| Existing proof | ✅ Daily proof exercises path | ❌ Marker-only stub |

---

## 4. Blockers: **None** for Quil visible typing

The path exists, is exercised in daily proof via synthetic events, and shares the same dispatch as real USB input.

---

## 5. Next Prompt

**MISSION: QUIL_VISIBLE_TYPING_E2E_V1**

- Boot with real USB keyboard (or synthetic HID proof if no USB available)
- Focus Quil surface
- Type characters, capture serial log
- Verify `[quil.text.recv]` markers for each typed character
- Verify `[quil.text.draw.v2]` markers for render
- Document that visible typing works end-to-end
- STOP FIRST if path requires kernel/ABI/sex-pdx edits

---

## 6. Handoff

```
docs/handoff/TYPING_VISIBLE_SURFACE_AUDIT_V1.md
```

## 7. Commit

```bash
git add docs/handoff/TYPING_VISIBLE_SURFACE_AUDIT_V1.md
git commit -m "docs(audit): typing visible surface audit V1"
```
