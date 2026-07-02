# SPINDLE_CLOSE_RELAUNCH_RESTORE_V1

**Date:** 2026-05-06
**Status:** Lifecycle proof — compile-time only, honest status
**Contract:** docs/handoff/SPINDLE_APP_CONTRACT_V1.md
**Previous:** SPINDLE_PROOF_COMMANDS_V1
**Next:** SPINDLE_COMPLETE_V1_AUDIT

---

## Summary

Added `close` command with honest lifecycle status:
- Reports session state as in-memory only
- No SexFiles persistence — history lost on close
- Relaunch = fresh state, no restore
- Close/relaunch requires kernel spawn + lifecycle integration
- Lifecycle marker emitted: `[spindle.lifecycle.close]`

---

## Close Behavior

```
sex> close
Spindle session closing.
  state:      in-memory only (no SexFiles persistence)
  surface:    WindowBuffer released on PD exit
  history:    not persisted (SexFiles bridge pending)
  relaunch:   fresh state, no restore available
Close/relaunch requires kernel spawn + lifecycle integration.
```

| Property | Status |
|----------|--------|
| History persistence | Pending (SexFiles bridge) |
| Relaunch restore | Unavailable |
| Stale surface IDs | N/A (no lifecycle mgmt) |
| Stale input protection | N/A (no HID delivery) |
| Fault containment | Fully preserved (no kernel weaken) |

---

## Spindle V1 Complete: 10 Commits

| # | Commit | Feature |
|---|--------|---------|
| 1 | `76893db` | Surface render scaffold |
| 2 | `8c01e37` | Line editor + input proof |
| 3 | `246ce43` | Scrollback ring |
| 4 | `4a856a6` | Command dispatch |
| 5 | `164425f` | History ring |
| 6 | `6467ac3` | Event ring |
| 7 | `f5afe48` | Session summary |
| 8 | `71a0412` | Apps + launch |
| 9 | `90b202c` | Proof commands |
| 10 | *(this)* | Close/lifecycle |

### Final Stats

| Metric | Value |
|--------|-------|
| Commits | 10 |
| Commands | 20 |
| Source lines | ~650 |
| Static BSS | ~115 KiB |
| Handoff docs | 10 |
| Kernel edits | **0** |
| sex-pdx edits | **0** |
| POSIX std/libc | **0** |
| All gates | **GREEN_MASTER** |
| All faults | **0** |

### All Pending Bridges

| Bridge | Blocked On |
|--------|-----------|
| SexFiles history persistence | Kernel spawn |
| Bell event bridge | Kernel spawn |
| Linen session object | Kernel spawn |
| App launch (4 targets) | Kernel spawn |
| Close/relaunch lifecycle | Kernel spawn |

**One STOP FIRST approval unblocks all 5.**

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| cargo check | PASS |
| entrypoint_build.sh | PASS |
| master_runtime_gate | **GREEN_MASTER** |
| Faults | **0** |

---

## Next Prompt

```
SPINDLE_COMPLETE_V1_AUDIT
```
