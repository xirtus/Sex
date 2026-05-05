# QUIL_PD_SPAWN_V1B

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Deliberately adds Quil as a booted protection-domain server with minimum
required capability. Quil boots, listens for PDX messages, and yields on
unknown messages. **No framebuffer writes, no editor logic, no surface
creation** — the shell retains sole ownership of Quil's frame/surface/placeholder
path (proven in V1A). No SLOT_DISPLAY grant in this phase.

---

## Files Changed

| File | Change |
|------|--------|
| `kernel/src/init.rs` | +4 lines (boot topology) |
| `servers/quil/src/main.rs` | Replace bare yield loop with PDX listen loop + proof markers |
| `docs/handoff/QUIL_PD_SPAWN_V1B.md` | New handoff doc |

---

## Boot Topology Change

### PD ID / Slot
| Property | Value |
|----------|-------|
| Domain ID | 9 |
| Spawn order | 9th (after sexstore at 8th) |
| module_paths position | 8th index (0-based) |
| MAX_DOMAINS | 1024 — no conflict |

### Kernel Initialization (init.rs)

Three insertions:

1. **Variable**: `let mut quil_id = 0;` (line 35)
2. **module_paths**: Added `"quil"` after `"sexstore"` (line 38)
3. **Spawn handler**: `domain_id == 9 { quil_id = id; }` with `[kernel.spawn.quil]` marker (lines 77-79)

```rust
} else if domain_id == 9 {
    quil_id = id;
    serial_println!("[kernel.spawn.quil] id={} path={}", id, path);
}
```

**No SLOT_DISPLAY capability grant.** Quil receives only default boot
capabilities (self message ring via slot 0, serial output via syscall).

---

## Capability Analysis

### Granted
| Capability | Reason |
|------------|--------|
| *(none)* | Quil only needs default boot caps: self message ring (slot 0), serial syscall |

### Explicitly Not Granted
| Capability | Reason |
|------------|--------|
| SLOT_DISPLAY | Display surface would conflict with silk-shell's existing Quil frame/placeholder ownership. Deferred to future editor phase. |
| SLOT_SHELL | No need to send messages to shell in stub phase. |
| SLOT_INPUT | No input handling. |
| SLOT_STORAGE | No persistence. |
| SLOT_AUDIO | No audio. |
| SLOT_SILKBAR | No silkbar interaction. |
| SLOT_SEXSTORE | No persistence. |

---

## Quil Server Behavior

### Boot Flow
```
_start()
├── [quil.boot]
├── [quil.no_fb_write]
└── loop:
    ├── pdx_listen_raw(0)           blocking receive
    ├── [quil.pdx.listen]           budget 8 — message received
    └── match msg.type_id:
        └── _ → [quil.unknown.yield] budget 8 — unknown message
```

### Design Decisions
- **No surface creation**: Silk-shell owns the Quil surface/frame/placeholder
  path (proven lifecycle-safe in V1A). Quil server does not call `0xEC` or `0xEF`.
- **No SLOT_DISPLAY**: Avoids capability escalation and surface ownership conflict.
  Can be added in a future editor phase.
- **PDX listen loop**: Uses `pdx_listen_raw(0)` — blocks until message arrives,
  internally yields when empty. Ready to handle future protocol messages.
- **Budget markers**: Standard proof marker pattern with static mut budget counters.
  `[quil.pdx.listen]` (8) fires on received messages. `[quil.unknown.yield]` (8)
  fires on unrecognized type_ids.

---

## Proof Markers

| Marker | Location | Budget | Condition |
|--------|----------|--------|-----------|
| `[quil.boot]` | `quil/src/main.rs:20` | 1 (once) | Server starts |
| `[quil.no_fb_write]` | `quil/src/main.rs:21` | 1 (once) | Confirms no framebuffer access |
| `[quil.pdx.listen]` | `quil/src/main.rs:31` | 8 | PDX message received |
| `[quil.unknown.yield]` | `quil/src/main.rs:42` | 8 | Unrecognized message type |
| `[kernel.spawn.quil]` | `init.rs:78` | 1 (once) | Kernel spawns Quil PD |

---

## Build Verification

```sh
./scripts/entrypoint_build.sh
# Result: ISO produced, no errors
```

Boot log should show:
- `✓ Spawned PD N: /servers/quil (Domain 9)`
- `[kernel.spawn.quil] id=N path=/servers/quil`
- `[quil.boot]`
- `[quil.no_fb_write]`

---

## Behavior Changes

| Area | Change |
|------|--------|
| Boot topology | Quil PD spawned as domain 9 |
| Quil server | Now listens for PDX messages instead of bare yield |
| Silk-shell F9 path | **Unchanged** — Quil surface/frame/placeholder still shell-owned |
| Capability grants | **None added** — Quil runs with default caps |
| Display protocol | **Unchanged** |
| Sex-pdx ABI | **Unchanged** |

---

## STOP FIRST Findings

| Condition | Finding |
|-----------|---------|
| Quil needs any cap beyond SLOT_DISPLAY | ✅ No caps granted at all |
| Display request requires new opcode/protocol | ✅ No display request |
| Kernel spawn pattern is ambiguous | ✅ Matches existing linen/sexstore pattern |
| PD id/slot assignment conflicts | ✅ Domain 9 is free |
| Quil server needs heap/std/libc/thread/time | ✅ No heap needed |
| Booting Quil destabilizes existing PD order | ✅ Appended after sexstore, no index changes |
| Placeholder surface ownership conflicts with shell | ✅ Quil does not create surface |

**No STOP FIRST conditions triggered.**

---

## Remaining Risks

| Risk | Mitigation |
|------|-----------|
| Quil consumes scheduler resource as idle PD | Minimal — PDX listen blocks/yields internally |
| Future editor phase needs SLOT_DISPLAY | Can be granted then as deliberate cap change |
| Future editor phase may conflict with shell placeholder | Shell placeholder is overwritten by real Quil content at that point |

---

## References

- `QUIL_SURFACE_STUB_PLAN_SPLIT_V1.md` — phase split rationale
- `QUIL_SURFACE_STUB_V1A.md` — shell-side lifecycle proof
- `A7_DISPLAY_CONFORMANCE_V1.md` — display protocol boundaries
- `kernel/src/init.rs` — lines 35, 38, 77-79 (Quil additions)
- `servers/quil/src/main.rs` — full server source
