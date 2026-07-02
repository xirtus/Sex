# SPINDLE_APP_LAUNCH_COMMANDS_V1

**Date:** 2026-05-06
**Status:** Launch commands proven — all targets honestly unavailable
**Contract:** docs/handoff/SPINDLE_APP_CONTRACT_V1.md
**Previous:** SPINDLE_LINEN_SESSION_OBJECT_V1
**Next:** SPINDLE_PROOF_COMMANDS_V1

---

## Summary

Added `apps` command and expanded `launch` to cover all four known app targets:
- `apps` — lists available apps from static table
- `launch <name>` — reports status for quil/linen/mesh/collar
- All targets are **honestly unavailable** — Spindle not kernel-spawned
- No POSIX exec, no fork, no host command execution

---

## Apps Command

```
sex> apps
Available apps (static):
  quil     text editor
  linen    object browser
  mesh     device topology
  collar   authority wallet
All targets unavailable: Spindle not kernel-spawned.
```

## Launch Command

| Target | Status | Output |
|--------|--------|--------|
| `launch quil` | Unavailable | "launch: all targets unavailable in V1." |
| `launch linen` | Unavailable | (same) |
| `launch mesh` | Unavailable | (same) |
| `launch collar` | Unavailable | (same) |
| `launch` (no arg) | Help | "launch: specify an app. Use 'apps' to list." |
| `launch foo` | Unknown | "launch: unknown target. Use 'apps' to list." |

---

## Files Changed

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +17 lines — apps command, expanded launch |
| `docs/handoff/SPINDLE_APP_LAUNCH_COMMANDS_V1.md` | NEW |

---

## Spindle V1 Final Command Set (13 Commands)

| # | Command | Status |
|---|---------|--------|
| 1 | `help` | Implemented |
| 2 | `clear` | Implemented |
| 3 | `status` | Implemented |
| 4 | `pd` | Implemented |
| 5 | `servers` | Implemented |
| 6 | `bell` | Pending (honest) |
| 7 | `files` | Pending (honest) |
| 8 | `apps` | Implemented |
| 9 | `launch <app>` | Unavailable (honest, 4 targets) |
| 10 | `history` | Implemented |
| 11 | `history clear` | Implemented |
| 12 | `events` | Implemented |
| 13 | `events clear` | Implemented |
| — | `session` | Implemented |

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| cargo check | PASS |
| entrypoint_build.sh | PASS |
| master_runtime_gate | **GREEN_MASTER** (6/6) |
| Faults | **0** |

---

## Next Prompt

```
SPINDLE_PROOF_COMMANDS_V1
```

---

## Contract Boundaries Preserved

- **No POSIX exec/fork/path lookup** — all commands are local string matching
- **No host command execution**
- **No new spawn ABI** — existing OP_APP_SURFACE_REQ used (unavailable)
- **No kernel edits**
- **No sex-pdx ABI edits**
- **No app manifest redesign**
- **Bounded command parser** — unchanged tokenizer
