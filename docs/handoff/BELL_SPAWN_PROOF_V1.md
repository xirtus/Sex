# BELL_SPAWN_PROOF_V1

**Status:** Complete — sexbell boots cleanly at domain 10 / PKEY 10. No faults. No regressions.
**Build:** `[SEXOS ENTRYPOINT] success` — ISO produced.
**Boot:** QEMU (30s timeout), serial log captured.
**Date:** 2026-05-05
**Depends on:** `BELL_BOOT_SPAWN_V1.md` (kernel spawn implementation)

---

## Summary

Runtime proof that sexbell boots correctly after `BELL_BOOT_SPAWN_V1`. Verified via QEMU boot with serial log capture. All expected markers present. No faults, no panics, no regressions in existing servers.

---

## Commands Run

| Step | Command | Result |
|------|---------|--------|
| Build | `./scripts/entrypoint_build.sh` | `[SEXOS ENTRYPOINT] success` |
| Boot | `./scripts/qemu_harness.sh --timeout 30` | Exit 124 (timeout — expected, clean boot) |
| Log | `/home/xirtus_arch/Documents/microkernel/logs/qemu-latest.log` | Captured |

---

## Fix Applied During Proof

**Problem:** sexbell (and quil) were not loading at boot, despite being compiled and present on the ISO.

**Root cause:** `limine.cfg` only listed 9 modules — quil and sexbell were missing as `MODULE_PATH` entries. Without a `MODULE_PATH` entry, Limine does not load the binary, and the kernel's spawn loop skips it.

**Fix:** Added `MODULE_PATH=boot:///servers/quil` and `MODULE_PATH=boot:///servers/sexbell` to `limine.cfg`. This is a non-functional config fix — no kernel logic, no ABI, no spawn table changes.

| File | Change |
|------|--------|
| `limine.cfg` | Added quil and sexbell MODULE_PATH entries (2 lines) |

---

## Proof Markers Found

| Marker | Line | Expected | Found |
|--------|------|----------|-------|
| `[kernel.spawn.sexbell]` | 699 | ✅ | ✅ id=10 path=/servers/sexbell |
| `[kernel.sexbell.cap]` | 715 | ✅ | ✅ self slot=12 |
| `[bell.boot]` | 963 | ✅ | ✅ sexbell entered listen loop |
| `[bell.unknown.reject]` | — | ✅ (absent) | ✅ No messages sent to Bell |

### Fault/Panic Check

| Marker | Found? | Verdict |
|--------|--------|---------|
| `fault.kill` | ❌ None | ✅ Clean |
| `panic` | ❌ None | ✅ Clean |
| `#GP` | ❌ None | ✅ Clean |
| `#PF` | ❌ None | ✅ Clean |
| `TRIPLE` | ❌ None | ✅ Clean |

---

## Boot Verification

### Spawn Sequence

| PD | Domain | Logged? |
|----|--------|---------|
| sexdisplay | 1 | ✅ |
| sexdrive | 2 | ✅ |
| silk-shell | 3 | ✅ |
| sexinput | 4 | ✅ |
| sexusb | 5 | ✅ |
| silkbar | 6 | ✅ |
| linen | 7 | ✅ |
| sexstore | 8 | ✅ |
| quil | 9 | ✅ |
| **sexbell** | **10** | **✅** |

Total: 10 PDs spawned. All expected.

### Cap Grant Verification

| Grant | Found? | Evidence |
|-------|--------|----------|
| SLOT_BELL self-cap (slot=12) | ✅ | `[kernel.sexbell.cap] self slot=12` |
| Any external Bell caps | ✅ None | No other Bell cap markers |

### Server Liveness

| Server | Activity in Log | Verdict |
|--------|-----------------|---------|
| quil | 21 log hits | ✅ Alive |
| sexstore | 16 log hits | ✅ Alive |
| sexdisplay | 33 log hits | ✅ Alive |
| sexbell | 3 log hits (spawn + cap + boot) | ✅ Alive |

---

## Scope Confirmation

| Area | Touched? | Evidence |
|------|----------|----------|
| `kernel/src/init.rs` | ❌ Not changed | Already correct from BELL_BOOT_SPAWN_V1 |
| `crates/sex-pdx/src/lib.rs` | ❌ Not changed | SLOT_BELL=12 unchanged |
| `servers/sexbell/src/main.rs` | ❌ Not changed | Stub behavior unchanged |
| `limine.cfg` | ✅ Added 2 MODULE_PATH entries | quil + sexbell module loading |
| `sexos_build_spec.toml` | ❌ Not changed | Already correct |
| sexdisplay/silk-shell/storage | ❌ Not touched | Unchanged |

---

## STOP FIRST Findings

| # | Condition | Check | Stop? |
|---|-----------|-------|-------|
| S1 | sexbell faults on boot | ✅ Clean boot, no panic | ❌ Not triggered |
| S2 | Quil/sexstore/display/shell regress | ✅ All 10 domains spawned | ❌ Not triggered |
| S3 | External caps granted | ✅ Only SLOT_BELL self-cap | ❌ Not triggered |
| S4 | OP_BELL_* parsing added | ✅ Stub rejects all (no messages sent) | ❌ Not triggered |
| S5 | Protocol activity before proof | ✅ No Bell protocol activity | ❌ Not triggered |
| S6 | limine.cfg change causes regression | ✅ Modules load in order, existing servers unchanged | ❌ Not triggered |

**All STOP FIRST conditions pass.**

---

## Final Verdict

```
╔══════════════════════════════════════════════════════╗
║            BELL SPAWN PROOF — PASS                  ║
╠══════════════════════════════════════════════════════╣
║ sexbell spawned:           ✅ domain 10/PKEY 10     ║
║ SLOT_BELL self-cap:        ✅ slot 12               ║
║ bell.boot marker:          ✅                       ║
║ No fault/panic:            ✅                       ║
║ All 10 PDs boot:           ✅                       ║
║ quil boots:                ✅ domain 9              ║
║ sexstore boots:            ✅ domain 8              ║
║ sexdisplay active:         ✅                       ║
║ No external Bell caps:     ✅                       ║
║ No Bell protocol activity: ✅                       ║
╚══════════════════════════════════════════════════════╝
```

---

## Next Phase Recommendation

**BELL_NOTIFY_PROOF_V1** or **BELL_RING_BUFFER_V1** — Add a test notification from an existing PD (e.g., silk-shell or Linen) to Bell to prove `OP_BELL_NOTIFY` route + `[bell.unknown.reject]` changes to `[bell.notify.recv]`. This would be the first protocol crossing.

---

## References

- `BELL_BOOT_SPAWN_V1.md` — kernel spawn implementation
- `BELL_BOOT_SPAWN_PLAN_V1.md` — spawn plan
- `BELL_NAMESPACE_COLLISION_AUDIT_V1.md` — namespace audit
- `BELL_SLOT_OPCODE_ASSIGNMENT_V1.md` — SLOT_BELL=12, OP_BELL_* 0xC0-0xC7
- `BELL_SERVER_STUB_V1.md` — sexbell crate
- `kernel/src/init.rs` — spawn table (domain 10 capture)
- `servers/sexbell/src/main.rs` — stub behavior
- `limine.cfg` — MODULE_PATH entries
- `/home/xirtus_arch/Documents/microkernel/logs/qemu-latest.log` — raw boot log

---

*End of BELL_SPAWN_PROOF_V1.md*
