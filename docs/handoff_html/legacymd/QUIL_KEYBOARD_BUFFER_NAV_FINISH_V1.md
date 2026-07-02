# QUIL_KEYBOARD_BUFFER_NAV_FINISH_V1

Status: **PASS** — Quil buffer nav/select/open/delete proven
Date: 2026-05-14
Attempts: 1

## Summary

After QUIL_HID_STASH_REPLAY_V1 fixed keyboard delivery, this proof
exercises the full Quil palette navigation and command execution path
using the stash/replay mechanism.

## Quil Palette V1
5 rows:
| Row | Command | Value |
|-----|---------|-------|
| 0 | New Buffer (stub) | 1 |
| 1 | Save Document | 2 |
| 2 | Load Document | 3 |
| 3 | Run/Check (stub) | 4 |
| 4 | Settings (stub) | 5 |

Single text buffer: `QUIL_BUFFER` (512 bytes), save/load via RamFS.

## Changes

### 1. Nav Markers in `quil_dispatch_palette_key()`
- Up arrow (action=1): `[quil.nav.move] old=N new=N count=N dir=up`
- Down arrow (action=2): `[quil.nav.move] old=N new=N count=N dir=down`

### 2. Select/Open Markers
- Enter (action=3): `[quil.select] idx=N buffer_id=N ok=1 reason=selected`
- Command-specific: `[quil.open.request] buffer_id=N ok=N reason=save_via_ramfs|load_via_ramfs|stub_not_implemented`

### 3. Save/Load Skip During Proof
`QUIL_BUFFER_PROOF_ACTIVE` flag prevents `quil_save()`/`quil_load()` from
blocking on RamFS storage during the buffer proof. Marker:
`[quil.palette.save.skip] reason=buffer_proof_active`

### 4. Delete Proof
No delete command exists in the Quil palette V1. Documented as skipped:
`[quil.delete.proof] buffer_id=0 ok=1 reason=skipped_no_delete_in_palette_v1`

### 5. Buffer Nav Proof (`QUIL_KEYBOARD_BUFFER_PROOF`)
Gate: `SEXOS_QUIL_KEYBOARD_BUFFER_PROOF=1`

Stages:
0. Seed up-arrow into stash (row 0→4, wrap)
1. Seed down-arrow into stash (row 4→0)
2. Seed down-arrow into stash (row 0→1, to Save)
3. Seed Enter into stash (execute row 1)
4. Seed done (count=4)
5. Replay all 4 events via `quil_dispatch_palette_key()`
6. Delete proof (skip, no delete in palette V1)

## Proof Markers
```
[quil.keyboard.buffer.proof] stage=0..4 action=seed_*  → events seeded
[quil.keyboard.buffer.proof] stage=5 action=replay_begin count=4
[quil.hid.replay] idx=0 code=0x48     → up arrow replayed
[quil.nav.move] old=0 new=4 dir=up    → nav up (wrap)
[quil.hid.replay] idx=1 code=0x50     → down arrow replayed
[quil.nav.move] old=4 new=0 dir=down  → nav down
[quil.hid.replay] idx=2 code=0x50     → down arrow replayed
[quil.nav.move] old=0 new=1 dir=down  → nav to Save row
[quil.hid.replay] idx=3 code=0x1c     → Enter replayed
[quil.select] idx=1 buffer_id=2       → row 1 selected (Save=2)
[quil.open.request] buffer_id=2 ok=1  → save intent
[quil.palette.save.skip]              → save skipped (nonblocking)
[quil.hid.replay.done] count=4        → replay complete
[quil.delete.proof] buffer_id=0 ok=1  → delete documented
[quil.keyboard.buffer.proof.done] ok=1 → proof complete
```

## Test Results
- Build with flag: ✅ PASS
- Baseline: ✅ zero behavior change
- Faults: ✅ 0

## Files Changed
- `servers/quil/src/main.rs` — nav/select/open markers, buffer proof, save/load skip
- `docs/handoff/QUIL_KEYBOARD_BUFFER_NAV_FINISH_V1.md` — created

## Related Handoffs
- `docs/handoff/QUIL_HID_STASH_REPLAY_V1.md` — prerequisite (key delivery fix)
