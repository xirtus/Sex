# BELL_NOTIFY_CLEANUP_V1

**Status:** Cleanup complete.
**Build:** `[SEXOS ENTRYPOINT] success`

---

## Files Changed

| File | Change |
|------|--------|
| `kernel/src/init.rs` | Removed 25-line scaffolding block (lines 179-203) |

**Not touched:**
- `servers/sexbell/src/main.rs` — unchanged, handler preserved
- `crates/sex-pdx/src/lib.rs` — unchanged
- `limine.cfg` — unchanged
- `sexos_build_spec.toml` — unchanged
- Any other file

---

## Scaffold Removed

The following was removed from `kernel/src/init.rs`:

```
// ── BELL_NOTIFY_PROOF_SCAFFOLDING ───────────────────────────────────
// REMOVAL PROMISE: This block is proof scaffolding only...
if sexbell_id != 0 {
    use crate::ipc::{DOMAIN_REGISTRY, messages::MessageType};
    let arg0: u64 = (0u64 << 0)  | (2u64 << 8)  | (0u64 << 16) | (0u64 << 24);
    let msg = MessageType::IpcCall {
        func_id:   sex_pdx::OP_BELL_NOTIFY,
        arg0, arg1: 0, arg2: 0, caller_pd: 0,
    };
    if let Some(pd) = DOMAIN_REGISTRY.get(sexbell_id) {
        unsafe { let _ = (*pd.message_ring).enqueue(msg); }
        serial_println!("[kernel.sexbell.notify.test] ...");
    }
}
```

**Removal confirmation:** `rg -n "kernel.sexbell.notify.test" kernel/src/init.rs` → **no results**. No kernel test enqueue remains.

---

## What Was Preserved

| Component | Status | Location |
|-----------|--------|----------|
| sexbell spawn (domain 10) | ✅ Preserved | `init.rs:82` — `[kernel.spawn.sexbell]` |
| SLOT_BELL self-cap | ✅ Preserved | `init.rs:174` — `[kernel.sexbell.cap]` |
| sexbell `[bell.boot]` | ✅ Preserved | `sexbell/src/main.rs:43` |
| `OP_BELL_NOTIFY` handler | ✅ Preserved | `sexbell/src/main.rs:49` — `match OP_BELL_NOTIFY` |
| `[bell.notify.recv]` | ✅ Preserved | `sexbell/src/main.rs:96` |
| `[bell.notify.downgrade]` | ✅ Preserved | `sexbell/src/main.rs:111` |
| `[bell.notify.ok]` | ✅ Preserved | `sexbell/src/main.rs:123` |
| `[bell.notify.reject]` | ✅ Preserved | `sexbell/src/main.rs:82` |
| `[bell.unknown.reject]` | ✅ Preserved | `sexbell/src/main.rs:138` |
| All 10 PD spawns | ✅ Preserved | Unchanged spawn order in init.rs |
| All other cap grants | ✅ Preserved | Unchanged |

---

## Build Result

```
[SEXOS ENTRYPOINT] success
```

---

## Proof: No Kernel Test Enqueue Remains

```bash
$ rg -n "kernel.sexbell.notify.test\|BELL_NOTIFY_PROOF_SCAFFOLDING" kernel/src/init.rs
# No results — scaffold fully removed

$ rg -n "sexbell" kernel/src/init.rs | head -10
36:    let mut sexbell_id = 0;
39:    ... "sexbell" ...
81:                            sexbell_id = id;
82:                            serial_println!("[kernel.spawn.sexbell] ...");
169:    // Bell self-cap: grant SLOT_BELL to sexbell ...
174:            pd.grant_capability(sex_pdx::SLOT_BELL, ...);
175:            serial_println!("[kernel.sexbell.cap] ...");

$ rg -n "OP_BELL_NOTIFY" servers/sexbell/src/main.rs
4:  use sex_pdx::{..., OP_BELL_NOTIFY};
49:             OP_BELL_NOTIFY => {
```

---

## Next Recommended Phase

**BELL_NOTIFY_NEGATIVE_PROOF_PLAN_V1** — Plan negative test for OP_BELL_NOTIFY with invalid enum values. Verify `[bell.notify.reject]` fires for bad payloads.

---

*End of BELL_NOTIFY_CLEANUP_V1.md*
