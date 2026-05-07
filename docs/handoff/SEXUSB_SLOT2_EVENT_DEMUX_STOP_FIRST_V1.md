# SEXUSB_SLOT2_EVENT_DEMUX_STOP_FIRST_V1

**Date:** 2026-05-07
**Status:** STOP FIRST — C2 attempt reverted, C1 restored

## 1. What Was Attempted

SEXUSB_SLOT2_EVENT_DEMUX_V1 attempted to replace the entire single-device
poll loop (~530 lines from lines 3655–4185) with a new two-device event
demux.  The replacement included:

- A new `HidDevice` struct with per-device ring/report state
- A `devices: [HidDevice; 2]` array replacing `single_bind`
- Population of `devices[1]` inside `if target_port_count > 1`
- Population of `devices[0]` after slot1 Configure Endpoint
- A complete rewrite of the poll loop with event matching, per-role
  dispatch, keyboard re-arm-before-IPC, tablet decode, and demux markers

## 2. Why 530-Line Replacement Was Too Broad

| Issue | Explanation |
|-------|-------------|
| **Scope violation** | The prompt said "Replace or extend current single-device binding." Replacing the ENTIRE poll loop is a broad USB refactor, explicitly forbidden. |
| **Single patch risk** | C2A (struct), C2B (event match), C2C (route) were all done in one replacement. Any single brace mismatch breaks the entire build. |
| **Untestable as one blob** | The old poll loop has subtle timing (keyboard re-arm ordering, skip_advance, burst spin) that must be preserved. A 530-line replacement cannot be reviewed incrementally. |
| **Brace mismatch** | The replacement had unclosed delimiters in the keyboard burst handler, confirming the risk of a monolithic replacement. |
| **Wasted diagnostic markers** | The per-device markers (demux.start, demux.event, demux.rearm) are useful but should be added AFTER the core demux works, not as part of the initial patch. |

## 3. Restoration Confirmed

| Check | Status |
|-------|--------|
| File restored from `main.rs.bak.slot2_poll_start` | ✅ |
| C1 patch re-applied | ✅ |
| `[sexusb.slot2.poll.start]` marker present | ✅ |
| `s2_intr_ring_va`, `s2_intr_report_va`, `s2_intr_dci` all in scope | ✅ |
| No `HidDevice` struct in file | ✅ (not yet) |
| No `devices` array in file | ✅ (not yet) |
| No demux loop code in file | ✅ (not yet) |
| Build passes | ✅ 1761 sectors, 0 errors |

## 4. Build Result

```
./scripts/entrypoint_build.sh → PASS, 1761 sectors, 0 new warnings
```

## 5. Smaller Next Plan

SEXUSB_SLOT2_EVENT_DEMUX should be split into three independent,
separately-testable patches:

### C2A — HidDevice Table Only (NO loop changes)

**Add** the `HidDevice` struct after `SingleHidBind`.  Declare
`let mut devices: [HidDevice; 2]` before the slot2 block.  Populate
`devices[1]` inside the slot2 block (using already-in-scope `s2_intr_*`
variables).  Populate `devices[0]` after slot1 Configure Endpoint.

**Do NOT touch the poll loop.**  `single_bind`, `intr_ring_va`, etc.
continue to work exactly as before.  The `devices` array is populated
but never read — pure data structure setup.

**Acceptance:**
- Build passes
- `[sexusb.slot2.poll.start]` still fires
- `[sexusb.hid.keyboard.continuous.start]` still fires
- Zero poll-loop changes
- ~60 lines added

### C2B — Event Match Helper (NO loop changes)

**Add** a helper function or inline code that, given a Transfer Event
(slot_id, ep), returns `Option<usize>` indicating which device (0 or 1)
the event belongs to.  Call it from a single diagnostic `serial_println!`
inside the existing poll loop, just after the current `slot == single_bind.slot_id`
check.  If the event matches slot2, log `[sexusb.hid.demux.match] slot=2`
but do NOT dispatch — fall through to the existing "bad event" path.

**Do NOT change dispatch.**  The old `if is_keyboard_device { ... } else if is_tablet_device { ... }`
dispatch stays exactly as-is.

**Acceptance:**
- Build passes
- `[sexusb.hid.demux.match] slot=2` appears when slot2 event arrives
- No behavior change — slot2 events still treated as "unrelated"
- ~20 lines added

### C2C — Route Slot2 Events (minimal dispatch change)

**After C2A and C2B pass independently**, add a third branch in the
dispatch: if the matched device is slot2 and its role is PointerTablet,
read the report from `devices[1].intr_report_va` and decode/forward
using the existing `decode_tablet_report()` + `OP_USB_MOUSE_REPORT` path.
Re-arm slot2's ring after IPC.

**The slot1 keyboard path stays completely untouched.**  Only the `else`
branch of the dispatch (which is currently dead code when slot1 is
keyboard) is replaced.

**Acceptance:**
- Build passes
- Slot2 idle reports decoded (zero bytes — expected on QEMU 11.0.0)
- `[sexusb.hid.slot2.report.idle]` fires
- Slot1 keyboard path unchanged
- ~40 lines changed in the else-branch of the dispatch

## 6. Files Changed (This Handoff)

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | Restored to C1 state (C2 reverted) |
| `docs/handoff/SEXUSB_SLOT2_EVENT_DEMUX_STOP_FIRST_V1.md` | Created |

## 7. Files NOT Touched

| Subsystem | Status |
|-----------|--------|
| kernel | ✅ |
| sex-pdx | ✅ |
| sexinput | ✅ |
| silk-shell | ✅ |
| sexdisplay | ✅ |

## 8. Final Verification

```bash
# Verify C1 is present
grep -c 'sexusb.slot2.poll.start' servers/sexusb/src/main.rs     # = 1

# Verify no demux residue
grep -c 'HidDevice\|devices:.*HidDevice\|demux' servers/sexusb/src/main.rs  # = 0 (or comments only)

# Build
./scripts/entrypoint_build.sh

# C1 gate
grep -c 'sexusb.slot2.poll.start' /tmp/slot2-poll-start.log      # ≥ 1
grep -cE 'panic|#PF|#GP' /tmp/slot2-poll-start.log               # = 0
```

---

*End of SEXUSB_SLOT2_EVENT_DEMUX_STOP_FIRST_V1.md*
