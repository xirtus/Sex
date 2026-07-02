# BELL_TO_SILKBAR_EVENT_PIPE_V1

**Status:** Implemented
**Date:** 2026-05-06
**Files changed:** 2 (+18 / -3 lines)

---

## Route Chosen

Existing Bell → SilkBar → sexdisplay route was already wired:
- `sexbell` processes `OP_BELL_NOTIFY` → stores in queue → bumps generation
- `silkbar` polls `OP_BELL_SUBSCRIBE` every ~2s → detects gen change → calls `OP_BELL_LIST`
- `silkbar` repacks lane counts → sends `UpdateKind::SetBellPresence` to sexdisplay
- `sexdisplay` renders gold/amber dot + count badge at `ModuleSlot::Bell`

Two gaps were filled:

### Gap 1 (sexbell): No events generated
Added a self-notify at boot: pushes one `Info/PASSIVE/Public` event into `BELL_QUEUE` with `caller_pd=0` (internal). Bumps generation so SilkBar detects the change on next poll.

**File:** `servers/sexbell/src/main.rs` (+12 lines)
- After `[bell.boot]` marker, calls `BELL_QUEUE.push(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)`
- On success: `bump_generation()` + `[bell.demo.boot]` marker
- On failure: `[bell.demo.boot.reject]` marker (queue full — unlikely at boot)

### Gap 2 (silkbar): Availability flag not set
SilkBar forwarded Bell's raw packed lane counts directly as `SetBellPresence.a`. This placed Bell's `lane_counts[1]` at the `flags` field position (bits 23:16). Since the demo event is in PASSIVE lane (lane 0), `lane_counts[1] = 0`, making `flags = 0`. Sexdisplay interprets `flags & 1 == 0` as "Bell unavailable" → renders dim dot.

Fix: repack the LIST reply properly for sexdisplay format:
- `bits 7:0 = total_visible` (from Bell bits 7:0)
- `bits 15:8 = redacted_count` (from Bell bits 63:56)
- `bits 23:16 = flags` (bit 0 = 1 since LIST succeeded)

**File:** `servers/silkbar/src/main.rs` (-0 / +3 lines)
- Extract `total_visible` from `packed & 0xFF`
- Extract `redacted_count` from `(packed >> 56) & 0xFF`
- Set `flags = 1` (bell_available)

---

## Proof Markers

From `serial.log`:
```
[bell.boot]                              — sexbell running
[bell.demo.boot] event_id=1              — demo event queued
[bell.subscribe.reply] gen=2             — generation bumped from 1→2
[silkbar.bell.gen.reply] gen=2 changed=1 — SilkBar detected change
[bell.list.item] event_id=1 final_lane=0 — one event visible
[bell.list.reply] total=1 lanes=[1 0 0 0 0 0] redacted=0
[silkbar.bell.poll.reply] total=1 redacted=0 flags=0x1 — forwarded with availability flag
[silkbar.bell.gen.reply] gen=2 changed=0 — subsequent polls: no change, no excess LIST
```

## Build / Runtime

- Build: `./scripts/entrypoint_build.sh` — PASS
- Gate: `./scripts/master_runtime_gate.sh` — GREEN_MASTER (all 5 gates)
- No faults, no panics, no regressions

## Invalid / Oversized Event Rejection

The existing sexbell validation gates apply to all `OP_BELL_NOTIFY` messages:
- `is_muted()` / `is_policy_muted()` check (reject before processing)
- `valid_category()`, `valid_privacy_level()`, `valid_redaction_class()` enum checks
- `urgency_hint > 3`, `action_count > 1`, `object_ref_count > 1` limits
- `check_spam_budget()` per-PD rate limiting
- Queue full → lowest-priority eviction or reject

No unbounded text fields exist in V1. All fields are fixed-width numeric.

## Remaining Bell V2 Risks

1. **sexdisplay message backlog**: SilkBar sends `OP_SILKBAR_UPDATE` to sexdisplay via async message ring. Cursor updates from silk-shell (0xEB) and other messages compete for queue slots. If backlog builds, Bell presence may be delayed by several seconds. Mitigation: separate priority lanes for status updates vs. cursor events.

2. **sexdisplay.bell.render budget**: The `[sexdisplay.bell.render]` log marker uses a `static mut` budget of 8. First 8 `SetBellPresence` updates are logged; subsequent ones are silently applied. Remove or increase budget once V2 stabilises.

3. **No popup/detail view**: Bell V1 is a presence dot + count badge. Clicking the Bell slot triggers `Action::OpenBell` but there's no panel UI — silk-shell creates a placeholder surface (ID 0x95). V2 needs a real event list panel.

4. **caller_pd=0 demotion in future**: The self-generated demo uses `caller_pd=0` which bypasses sender validation. In V2, a self-notify should go through the same validation path as external callers, or use a dedicated internal opcode.

5. **No negative test for oversized event**: Need to verify that sending `category=6` (invalid) is rejected with `[bell.notify.reject] reason=invalid_category`. This is covered by existing unit logic but no runtime gate checks it.

## Files Changed

```
servers/sexbell/src/main.rs     +12  (demo self-notify at boot)
servers/silkbar/src/main.rs     -3/+6 (LIST reply repacking with availability flag)
```

No sex-pdx ABI changes. No kernel edits. No renderer primitives. No persistence.
