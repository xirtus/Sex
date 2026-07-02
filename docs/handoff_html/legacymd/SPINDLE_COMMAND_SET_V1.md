# SPINDLE_COMMAND_SET_V1

**Date:** 2026-05-06
**Status:** Command set finalized — 25 commands, all GREEN_MASTER
**Previous:** SPINDLE_DISPLAY_SURFACE_V1

---

## Final Command Set: 25 Commands

| # | Command | Category | Status |
|---|---------|----------|--------|
| 1 | `help` | Core | Implemented |
| 2 | `clear` | Core | Implemented |
| 3 | `status` | Core | Implemented |
| 4 | `about` | Core | **New** — version, PD, surface, session info |
| 5 | `pd` | Runtime | Implemented |
| 6 | `servers` | Runtime | Implemented |
| 7 | `proof` | Runtime | Implemented |
| 8 | `proof boot` | Runtime | Implemented |
| 9 | `proof input` | Runtime | Implemented |
| 10 | `proof display` | Runtime | Implemented |
| 11 | `proof storage` | Runtime | Implemented |
| 12 | `faults` | Runtime | Implemented |
| 13 | `route` | Diagnostic | **New** — input/surface route info |
| 14 | `input` | Diagnostic | **New** — keyboard input status |
| 15 | `history` | Storage | Implemented |
| 16 | `history clear` | Storage | Implemented |
| 17 | `session` | Storage | Implemented |
| 18 | `events` | Events | Implemented |
| 19 | `events clear` | Events | Implemented |
| 20 | `apps` | Apps | Implemented |
| 21 | `launch <app>` | Apps | Implemented (4 targets) |
| 22 | `bell` | Bridge | Pending cap grant |
| 23 | `files` | Bridge | Pending cap grant |
| 24 | `close` | Lifecycle | Implemented |
| 25 | `unknown` | — | Auto-handled |

---

## New Commands (Phase 8)

| Command | Output |
|---------|--------|
| `about` | Version, source lines, PD domain, surface ID, session kind, bridge status |
| `route` | Input route (sexinput→silk-shell→SLOT_SPINDLE→PD12), surface route, FB gating |
| `input` | HID event format, scancode table, line editor bounds, real delivery status |

---

## Files Changed

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +30 lines — about, route, input commands; updated help |
| `docs/handoff/SPINDLE_COMMAND_SET_V1.md` | NEW |

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| entrypoint_build.sh | PASS |
| master_runtime_gate | **GREEN_MASTER** (6/6) |
| Faults | **0** |

---

## Spindle Terminal: 8/8 Phases Complete

| Phase | Commit | Status |
|-------|--------|--------|
| 1: SLOT_SPINDLE | `0ed3085` | ✅ |
| 2: Silk-shell routing | `f2b67e7` | ✅ |
| 3: Real keyboard input | `e5f1796` | ✅ |
| 4: SexFiles persistence | `bc5fbf0` | ✅ (guarded) |
| 5: Bell bridge | `22a1190` | ✅ (guarded) |
| 6: Linen .spn | `97633f3` | ✅ (guarded) |
| 7: Display surface | `39da006` | ✅ |
| 8: Command set | *(this)* | ✅ |
