# SPINDLE_BOOT_MODULE_WIRING_V1

**Date:** 2026-05-06
**Status:** Wired — Spindle spawned as PD 12, GREEN_MASTER, 0 faults
**Previous:** SPINDLE_COMPLETE_V1_AUDIT
**Approval:** Kernel edit (init.rs module_paths only) — STOP FIRST approved

---

## Summary

Wired Spindle into the kernel boot spawn sequence:
- Added to `kernel/src/init.rs` module_paths as entry 12
- Spawned as PD 12 (Domain ID 12), PKU key 12
- Framebuffer access guarded behind proof gate (no page faults)
- Normal spawn: serial-only idle loop
- Serial markers: `[spindle.boot]`, `[spindle.ready]`, `[kernel.spawn.spindle]`
- All existing PDs unaffected (domains 1-11 unchanged)

---

## Files Changed

| File | Change | Diff |
|------|--------|------|
| `kernel/src/init.rs` | +3 lines — spindle_id var, module_paths entry, domain 12 match | +3 |
| `apps/spindle/src/main.rs` | Restructured — FB access guarded behind proof gate | ~30 lines reformatted |
| `docs/handoff/SPINDLE_BOOT_MODULE_WIRING_V1.md` | NEW | — |

### Kernel Diff

```diff
+    let mut spindle_id = 0;
-    let module_paths = ["sexdisplay", ..., "sexfiles"];
+    let module_paths = ["sexdisplay", ..., "sexfiles", "spindle"];
+    } else if domain_id == 12 {
+        spindle_id = id;
+        serial_println!("[kernel.spawn.spindle] id={} path={}", id, path);
```

### Spindle Safety Guard

Framebuffer access (WindowBuffer, font::draw_str, etc.) is now inside `if INPUT_PROOF_ENABLED { ... }`. Normal spawn without `SEXOS_SPINDLE_INPUT_PROOF=1` skips all FB access — serial-only idle loop.

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| entrypoint_build.sh | **PASS** |
| master_runtime_gate | **GREEN_MASTER** (6/6) |
| Faults | **0** |
| Spindle loaded | PD 12, Domain 12, PKU 12 |
| Page faults | **0** |

### Serial Log Evidence

```
limine: Loading module `boot:///apps/spindle`...
pd: Creating domain for /apps/spindle (Domain ID 12)...
loader: Loading ELF /apps/spindle (PKU Key 12)...
 Spawned PD 12: /apps/spindle (Domain 12)
[kernel.spawn.spindle] id=12 path=/apps/spindle
[spindle.boot]
[spindle.surface.req] pd=12 kernel_spawned=1
[spindle.ready]
```

---

## Spindle V1 Completion: 100%

| Metric | Before | After |
|--------|--------|-------|
| Kernel spawn | ❌ STOP FIRST | ✅ PD 12 |
| PDX slot | ❌ None | ✅ Domain 12, self-ring slot 0 |
| Serial output | ❌ Compile-only | ✅ `[spindle.ready]` in log |
| Completion | 95% | **100%** |

---

## What This Unblocks

With kernel spawn active, the following bridges can now be wired (separate missions):

| Bridge | Server | Wiring Needed |
|--------|--------|---------------|
| Silk-shell HID forwarding | silk-shell | SURFACE_ID_SPINDLE + key route |
| SexFiles history persistence | sexfiles | RamFS PDX calls |
| Bell event bridge | sexbell | OP_BELL_NOTIFY calls |
| Linen session object | linen | OP_LINEN_CREATE_OBJECT calls |
| App surface request | silk-shell | OP_APP_SURFACE_REQ |

---

## Files NOT Changed (Per Mission Rules)

| File | Reason |
|------|--------|
| `crates/sex-pdx/` | No ABI changes |
| `kernel/src/scheduler.rs` | No scheduler edits |
| `kernel/src/memory/` | No memory manager edits |
| `kernel/src/syscall.rs` | No syscall edits |
| `kernel/src/interrupts/` | No interrupt edits |
| `servers/silk-shell/` | No surface routing yet |
| `servers/sexfiles/` | No persistence wiring yet |

---

## Contract Boundaries Preserved

- **No privilege escalation** — Spindle is a normal userland PD
- **No framebuffer access** — FB guarded behind proof gate
- **sexdisplay sole FB writer** — Spindle never touches FB in normal spawn
- **No scheduler/memory/syscall changes** — init.rs only
- **Existing PD IDs unchanged** — domains 1-11 identical
- **No ABI hash changes** — sex-pdx untouched
