# SPINDLE_BELL_EVENTS_V1

**Date:** 2026-05-06
**Status:** Local event ring proven — Bell bridge pending kernel spawn
**Contract:** docs/handoff/SPINDLE_APP_CONTRACT_V1.md
**Previous:** SPINDLE_SEXFILES_HISTORY_V1
**Next:** SPINDLE_LINEN_SESSION_OBJECT_V1

---

## Summary

Added a local bounded event ring with honest Bell bridge status:
- Fixed 32-entry ring buffer (32 × 80 bytes = 2.5 KiB BSS)
- Four event kinds: CmdOk, CmdFail, CmdUnknown, Info
- Recognized commands record CmdOk events automatically
- Unknown commands record CmdUnknown events automatically
- `events` command displays recent events with kind prefixes
- `events clear` command resets the ring
- Bell bridge is **pending** — honest status reported

---

## Bell Bridge Status: PENDING

Bell API exists in sex-pdx (8 opcodes via SLOT_BELL=12) but Spindle is not kernel-spawned and cannot make PDX calls.

```
Bell bridge pending (Spindle not kernel-spawned).
```

### Exact Missing Bridge

| Prerequisite | STOP FIRST? |
|-------------|-------------|
| Kernel spawn (add to init.rs module_paths) | YES |
| PDX slot allocation (SLOT_SPINDLE) | YES |
| `pdx_call(SLOT_BELL, OP_BELL_NOTIFY, ...)` | YES (requires spawn) |
| Bell sender capability grant | YES |

Until approved: local event ring only, no cross-PD Bell communication.

---

## Event Ring

| Parameter | Value |
|-----------|-------|
| Max events | 32 |
| Max line bytes | 80 (EV_BYTES) |
| Event kinds | CmdOk, CmdFail, CmdUnknown, Info |
| Storage | Static BSS — no allocation |
| Overflow | Wraps; oldest overwritten |

### Event Recording

- **Recognized commands** → `EvKind::CmdOk` + command name
- **Unknown commands** → `EvKind::CmdUnknown` + command name
- **CmdFail/Info** → reserved for future error paths and status messages

### Events Display

```
Event log (most recent first):
  [OK]    help
  [OK]    status
  [????]  asdf
  [OK]    clear
Bell bridge pending (Spindle not kernel-spawned).
```

---

## New Commands

| Command | Output |
|---------|--------|
| `events` | Lists recent events with kind prefix, then Bell pending status |
| `events clear` | Resets event ring |

---

## Files Changed

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +45 lines — EventRing struct, events commands, dispatch recording |
| `docs/handoff/SPINDLE_BELL_EVENTS_V1.md` | NEW |

## Files NOT Changed

| File | Reason |
|------|--------|
| `kernel/src/init.rs` | STOP FIRST |
| `crates/sex-pdx/` | STOP FIRST (Bell opcodes exist, can't use without spawn) |
| `servers/sexbell/` | No protocol changes |
| `servers/silk-shell/` | No routing changes |

---

## Proof Gate

Existing 20 stages pass. Event recording verified:
- Recognized commands push CmdOk events
- Unknown commands push CmdUnknown events
- Events stage deferred (requires kernel spawn for runtime verification)
- Status: "events: pending" noted in proof done marker

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| cargo check | PASS (1 warning: unused variable) |
| entrypoint_build.sh | PASS |
| master_runtime_gate | **GREEN_MASTER** (6/6) |
| Faults | **0** |

---

## Next Prompt

```
SPINDLE_LINEN_SESSION_OBJECT_V1
```

---

## Contract Boundaries Preserved

- **No fake Bell** — local event ring only, honest pending status
- **No kernel edits** — no spawn, no PDX slot
- **No sex-pdx ABI edits**
- **No sexbell protocol changes**
- **No popup rendering** — Spindle never writes outside its window
- **No sexdisplay policy changes**
- **Bounded event store** — 32 entries × 80 bytes, static BSS
