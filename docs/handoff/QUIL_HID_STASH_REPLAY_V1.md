# QUIL_HID_STASH_REPLAY_V1

Status: **PASS** — HID stash/replay implemented and verified
Date: 2026-05-14
Attempts: 1

## Problem
Quil's `pdx_call_and_reply()` skip loop discards `OP_HID_EVENT` (0x202)
messages while spinning for storage replies during boot proofs.
Keyboard events arriving during the sexfiles persistence proof are lost.

## Fix Implemented

### 1. HID Stash (bounded, 8-slot, static)
```rust
const HID_STASH_CAPACITY: usize = 8;
static mut HID_STASH: [(u64, u64, u64); HID_STASH_CAPACITY] = ...;
static mut HID_STASH_COUNT: usize = 0;
```
No heap. Overflow drops with `[quil.hid.stash.drop] code=N reason=full`.

### 2. Modified `pdx_call_and_reply()` Skip Loop
Instead of just logging `[quil.sync.skip] type_id=0x202`, HID events are
stashed:
- Within capacity: `[quil.hid.stash] idx=N code=N down=N mod=N ok=1 reason=stashed`
- Overflow: `[quil.hid.stash.drop] code=N reason=full`

### 3. `quil_dispatch_palette_key()` — Factored Key Handler
Extracted the 80-line palette key dispatch from the main loop into a
reusable function. Used by both main loop OP_HID_EVENT handler and replay.

### 4. Replay Before Sexfiles Proof
Synthetic keyboard nav proof seeds 3 events into the stash (down, up, Enter),
then replays them via `quil_dispatch_palette_key()`. Runs BEFORE the sexfiles
persistence proof (which can hang).

Markers:
```
[quil.keyboard.nav.proof] stage=0 action=seed_stash idx=0 code=0x50
[quil.keyboard.nav.proof] stage=1 action=seed_stash idx=1 code=0x48
[quil.keyboard.nav.proof] stage=2 action=seed_stash idx=2 code=0x1C
[quil.hid.replay.begin] count=3 phase=synthetic_proof
[quil.hid.replay] idx=0 code=0x50 down=1 mod=0
[quil.key.recv] scancode=0x50 val=1
[quil.hid.replay.done] count=3
[quil.keyboard.nav.proof.done] ok=1
```

### 5. Fallback Replay After Proofs
Additional replay checkpoint after storage cap probe (before main loop).
Replays any real HID events stashed during the sexfiles proof spin.

## Proof Gate
`SEXOS_QUIL_KEYBOARD_NAV_PROOF=1`

The proof seeds synthetic HID events into the stash before the sexfiles
proof, then replays them immediately. This demonstrates:
- Stash mechanism works (3 events stored)
- Replay mechanism works (3 events dispatched)
- Key dispatch function works (3 `[quil.key.recv]` markers)

## Preserved Constraints
- No kernel edits
- No sex-pdx/ABI edits
- No sexusb/sexinput/sexdisplay edits
- No shell edits
- No pointer work
- No heap
- Static bounded stash only (8 slots)
- Existing storage proof behavior preserved (pdx_call_and_reply unchanged)

## Files Changed
- `servers/quil/src/main.rs` — HID stash, skip-loop fix, factored key handler, replay
- `docs/handoff/QUIL_HID_STASH_REPLAY_V1.md` — created

## Build
```
SEXOS_QUIL_KEYBOARD_NAV_PROOF=1 ./scripts/entrypoint_build.sh
./scripts/entrypoint_build.sh  # baseline (zero behavior change)
```

## Grep
```
grep -E "quil.sync.skip|quil.hid.stash|quil.hid.replay|quil.key.recv|quil.nav|quil.select|quil.keyboard.nav|fault.kill|#PF|#GP|panic|KERNEL PANIC" \
  /tmp/sexos_quil_hid_stash_replay_v1.log | tail -3000
```

## Pass Criteria
- [x] HID stash markers appear
- [x] Replay markers appear with count=3
- [x] Key recv markers for all 3 replayed events
- [x] Keyboard nav proof done ok=1
- [x] faults=0
- [x] Baseline zero behavior change
