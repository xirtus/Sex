# SEXNET_SPAWN_READINESS_AUDIT_V1

**Status:** PASS REVIEW ONLY — sexnet is build-ready, can be spawned passively.
**Date:** 2026-05-16

---

## Readiness Table

| Check | Status | Notes |
|-------|--------|-------|
| Build integration | ✅ | `scripts/rust_build.sh` line 46 |
| Source size | 139 lines | Small server |
| Boot spawn | ❌ | Not in `kernel/src/init.rs` |
| PDX listen loop | ✅ | `pdx_listen_raw(0)` standard pattern |
| Uses alloc/heap | ⚠️ | `LockedHeap` — needs heap init in PD |
| Hardware dependency | ✅ | Mock AP scan only, no real NIC |
| Blocking risk | ✅ | Non-blocking listen loop |
| Panic handler | ✅ | Present |
| Opcodes | 6 | GET_STATUS, SCAN_WIFI, CONNECT, DISCONNECT, VPN_UP/DOWN, GET_IP |
| Markers | ❌ | No boot marker yet |

---

## Blockers

| Blocker | Severity | Fix |
|---------|----------|-----|
| Not in init.rs | High | Add sexnet to module_paths + capture PD + grant self-cap |
| Uses alloc | Medium | Needs LockedHeap::init() in _start() |
| No proof markers | Low | Add [sexnet.boot] marker |

---

## Recommended: **A — SEXNET_PASSIVE_SPAWN_V1**

Spawn sexnet as passive/status PD. No network packets. No NIC dependency. Browser still network=0.

Required changes:
- `kernel/src/init.rs`: add sexnet to module_paths, capture domain, grant self-cap
- `servers/sexnet/src/main.rs`: add [sexnet.boot] marker, init allocator
- Browser: no changes (still network=0)

---

## STOP FIRST Boundaries (all pass)

| Boundary | Status |
|----------|--------|
| Kernel ABI edit | ❌ No — standard spawn pattern |
| Boot module layout risk | ❌ Low — append to end |
| Blocking hardware wait | ❌ No — mock only |
| Scheduler stall | ❌ No — standard listen |
| Browser network=1 | ❌ No — stays 0 |

---

## Next: SEXNET_PASSIVE_SPAWN_V1

## Commit
```bash
git add docs/handoff/SEXNET_SPAWN_READINESS_AUDIT_V1.md
git commit -m "docs(net): sexnet spawn readiness audit V1"
```
