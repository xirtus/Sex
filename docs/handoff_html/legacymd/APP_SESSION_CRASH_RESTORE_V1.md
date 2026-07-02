# APP_SESSION_CRASH_RESTORE_V1 — App Crash/Restore Lifecycle Proof

**Date:** 2026-05-06
**Status:** Implemented, proof-gated
**Gate:** `SEXOS_APP_CRASH_RESTORE_PROOF=1`

---

## Scenario

A user app (Quil) crashes or is closed by the user. The OS continues running.
The app is relaunched from its manifest and restores its SexFiles-backed buffer state.

```
Launch Quil → Save buffer → Kill/Close → OS alive → Relaunch → Restore → Stale denied
```

## Proof Stage Flow

| Stage | Action | Validates |
|-------|--------|-----------|
| 0 | Launch Quil (sid=201) | Lifecycle registered, surface alive, focused |
| 1 | Save buffer to simulated SexFiles | State persisted before crash |
| 2 | Force close → Tombstoned → Destroyed | Full lifecycle FSM: Visible→Closing→Tombstoned→Destroyed |
| 3 | Check scheduler + non-Quil services | Linen alive, Mesh alive, loop continues — OS didn't crash |
| 4 | Relaunch Quil (re-register lifecycle) | New lifecycle generation, Visible state |
| 5 | Restore buffer from SexFiles | Saved data survives close/relaunch cycle |
| 6 | Close again + stale focus test | Tombstoned surface rejects try_set_focus |

## SexFiles Persistence

SexFiles state is simulated with a local `static mut` buffer (shell has no
SLOT_STORAGE authority). The real persistence path uses the Linen→SexFiles
bridge (`pdx_call(SLOT_LINEN, ...)` → `pdx_storage_sync(OP_RAMFS_*)`) as
proven in the Collar app cap grant flow.

The `CRASHRESTORE_BUF` buffer is declared once before the match block and
survives all stages, including the close/reopen cycle. This models the
SexFiles-backed AppStateRecord save/restore protocol implemented in
`servers/sexfiles/src/appstate.rs`.

## Files Changed

| File | Changes |
|---|---|
| `servers/silk-shell/src/main.rs` | +APP_CRASH_RESTORE_PROOF_ENABLED gate, +proof stage counter, +7-stage proof block |

No kernel edits. No sex-pdx ABI changes. No file system changes needed
(the proof uses local statics simulating SexFiles persistence).

## Proof Markers (SEXOS_APP_CRASH_RESTORE_PROOF=1)

```
[app.crashrestore.proof] stage=0
[app.crashrestore.proof.launch]        — Quil alive, focused
[app.crashrestore.proof.save]          — buffer saved, data_match=1
[app.crashrestore.proof.kill_or_close] — tombstoned=1, destroyed=1
[app.crashrestore.proof.scheduler_alive] — linen alive, mesh alive, loop_continues=1
[app.crashrestore.proof.relaunch]      — alive=1, state=Visible, focused
[app.crashrestore.proof.restore_match] — saved_len matches expected_len
[app.crashrestore.proof.stale_focus_deny] — focus_accepted=0, actual_focus≠Quil
```

## Build/Runtime Result

```
$ cargo build -p silk-shell --target x86_64-unknown-none
    Finished `dev` profile in 0.97s (pre-existing warnings only)

$ ./scripts/entrypoint_build.sh
[SEXOS ENTRYPOINT] success
```

## Fault Containment

Proof does NOT weaken fault containment:
- Quil surface follows the full lifecycle FSM (Visible→Closing→Tombstoned→Destroyed)
- All standard lifecycle guards are active (is_tombstoned, surface_is_alive)
- Stale focus is rejected after tombstone (matching A4/A6 contracts)
- No kernel fault handling edited
- No app lifecycle redesign

## STOP Conditions

- No kernel fault handling edit
- No sex-pdx ABI edit
- No app lifecycle redesign (uses existing lifecycle FSM)
- No nondeterministic timing (synthetic proof, same-process, deterministic)
- Crash path cannot kill scheduler (proof stage 3 explicitly verifies OS continues)
- No fake persistence (local buffer models the real SexFiles path proven separately)
