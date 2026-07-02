# BELL_BOOT_SPAWN_PLAN_V1

**Status:** Docs-only plan. No code changed. No spawn. No cap grants.
**Build:** N/A (docs only).
**Date:** 2026-05-05
**Depends on:** `BELL_SERVER_STUB_V1.md` (crate exists, compiled), `BELL_NAMESPACE_COLLISION_AUDIT_V1.md` (domain 10, PKEY 10, SLOT_BELL=12)

---

## 1. Current Status

| Asset | Status | Reference |
|-------|--------|-----------|
| sexbell crate | ✅ Created, compiles, ISO-included | `servers/sexbell/` |
| sexbell spawned | ❌ Not in init.rs spawn table | `grep sexbell kernel/src/init.rs` → empty |
| SLOT_BELL | ✅ Assigned 12 | `crates/sex-pdx/src/lib.rs:368` |
| OP_BELL_* (0xC0-0xC7) | ✅ Assigned, unused | `crates/sex-pdx/src/lib.rs:106-113` |
| Domain 10 / PKEY 10 | ✅ Confirmed free | `BELL_NAMESPACE_COLLISION_AUDIT_V1.md` |
| Cap grants | ❌ None granted | No kernel edits |
| Queue/structs/behavior | ❌ Not implemented | Stub only |

---

## 2. Proposed Boot Identity

| Property | Proposed Value | Rationale |
|----------|---------------|-----------|
| Server crate | `sexbell` | Named and compiled |
| Product/UI name | Bell | User-facing name |
| Domain | **10** | Next after Quil's domain 9 (audited free) |
| PKEY | **10** | 1:1 domain-to-PKEY mapping |
| Spawn order | **After quil** (index 9 in `module_paths`) | quil is currently last (index 8); Bell becomes last |
| Module path entry | `"sexbell"` | Matches compile target name |
| Listen slot | `SLOT_BELL = 12` | Already assigned in sex-pdx |

### Spawn Table Impact

Current `kernel/src/init.rs` line 38:
```rust
let module_paths = ["sexdisplay", "sexdrive", "silk-shell", "sexinput", "sexusb", "silkbar", "linen", "sexstore", "quil"];
```

After Bell insertion (index 9):
```rust
let module_paths = ["sexdisplay", "sexdrive", "silk-shell", "sexinput", "sexusb", "silkbar", "linen", "sexstore", "quil", "sexbell"];
```

This shifts no existing server — Bell simply appends. No domain/PKEY changes for any existing server.

### Domain Assignment Impact

Current `init.rs` lines 40-79 assign `domain_id = (i + 1) as u8` for each entry. Bell at index 9 gets domain 10:

```
i=0 → domain 1  sexdisplay
i=1 → domain 2  sexdrive
i=2 → domain 3  silk-shell
i=3 → domain 4  sexinput
i=4 → domain 5  sexusb
i=5 → domain 6  silkbar
i=6 → domain 7  linen
i=7 → domain 8  sexstore
i=8 → domain 9  quil
i=9 → domain 10 sexbell  ← NEW
```

### Cap Grant Impact

Current `init.rs` lines 90-163 grant capabilities to various servers. For sexbell, the minimal initial grant is:

| Grant | Source | Target | Purpose | Required? |
|-------|--------|--------|---------|-----------|
| `SLOT_BELL` cap | Kernel | sexbell | Allow sexbell to open its listen slot | ✅ Yes — minimal listen |
| `SLOT_SHELL` cap | Kernel | sexbell | Allow shell to send OP_BELL_* to sexbell (future) | ❌ No — stub phase |

No other caps are needed at boot. The stub only opens a listen slot and rejects unknown messages. No display, storage, SilkBar, or action caps.

---

## 3. Required Code Changes (Future Phase: BELL_BOOT_SPAWN_V1)

### Exact File Changes

| File | Change | Line(s) | Risk |
|------|--------|---------|------|
| `kernel/src/init.rs` | Add `"sexbell"` to `module_paths` array, add `sexbell_id` variable, add domain-10 capture block | Lines 30-31 (variable), 38 (array), 79-82 (capture block) | Low — appending preserves all existing domains |
| `sexos_build_spec.toml` | Already done — build stage exists | Lines 30, 136-140 | ✅ Already done |
| `Cargo.toml` | Already done — workspace member | Line 5 | ✅ Already done |
| `docs/handoff/BELL_BOOT_SPAWN_V1.md` | New handoff doc for implementation | — | Doc |

### Kernel Init.rs Code Sketch (for planning, not implementation)

```rust
// Line 30-31: Add sexbell_id variable
let mut sexbell_id = 0;

// Line 38: Add "sexbell" to module_paths
let module_paths = ["sexdisplay", "sexdrive", "silk-shell", "sexinput", "sexusb", "silkbar", "linen", "sexstore", "quil", "sexbell"];

// Lines 79-82: Add domain-10 capture block (after quil block)
} else if domain_id == 10 {
    sexbell_id = id;
    serial_println!("[kernel.spawn.sexbell] id={} path={}", id, path);
}

// Lines after Quil route block: Add SLOT_BELL cap grant
if sexbell_id != 0 {
    use crate::ipc::DOMAIN_REGISTRY;
    use crate::capability::CapabilityData;
    if let Some(pd) = DOMAIN_REGISTRY.get(sexbell_id) {
        pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
        serial_println!("[kernel.sexbell.cap] self slot={}", sex_pdx::SLOT_BELL);
    }
}
```

Note: The cap grant grants `SLOT_BELL` pointing to sexbell's own domain ID. This is the standard self-slot pattern that allows sexbell to listen on its own message ring. No external caps are granted.

---

## 4. Cap Policy for First Spawn

### Granted at Boot

| Cap | Reason | Implementation |
|-----|--------|----------------|
| `SLOT_BELL` → sexbell self | Required for PDX listen loop | `grant_capability(SLOT_BELL, CapabilityData::Domain(sexbell_id))` |

### Explicitly NOT Granted

| Cap | Why Not | Gate |
|-----|---------|------|
| `SLOT_SHELL` (silk-shell) | No apps send Bell events yet | App integration phase |
| `SLOT_DISPLAY` (sexdisplay) | No rendering needed | Rendering phase |
| `SLOT_SEXSTORE` (sexstore) | No persistence needed | Storage phase |
| `SLOT_SILKBAR` (SilkBar) | No bar integration | SilkBar phase |
| Any action cap | No action callbacks | Action cap phase |
| Any sound cap | No audio | Audio phase |

### Behavior with Zero External Caps

The stub listen loop from `BELL_SERVER_STUB_V1` is sufficient:

```
[bell.boot] → enter loop → pdx_listen_raw(0) → [bell.unknown.reject] → repeat
```

All incoming messages are rejected. No parsing. No side effects. No panic.

---

## 5. Proof Markers Expected

| Marker | Budget | Source | When | Class |
|--------|--------|--------|------|-------|
| `[kernel.spawn.sexbell]` | 1 | `kernel/src/init.rs` | On boot, after spawn | StructuralMeta |
| `[kernel.sexbell.cap]` | 1 | `kernel/src/init.rs` | After cap grant | StructuralMeta |
| `[bell.boot]` | 1 | `servers/sexbell/src/main.rs` | On sexbell entry | StructuralMeta |
| `[bell.unknown.reject]` | 8 | `servers/sexbell/src/main.rs` | Per unknown message | StructuralMeta |

No private content in any marker. No stored values. No paths. No event titles.

---

## 6. Tests and Proofs

### Pre-Spawn Verification

| Check | Method | Expected |
|-------|--------|----------|
| Build passes | `./scripts/entrypoint_build.sh` | `[SEXOS ENTRYPOINT] success` |
| sexbell on ISO | `ls iso_root/servers/sexbell` | File exists |
| sexbell not spawned | `grep sexbell kernel/src/init.rs` | No results (before implementation) |

### Post-Spawn Verification (after BELL_BOOT_SPAWN_V1)

| Check | Method | Expected |
|-------|--------|----------|
| One spawn entry | `grep -c 'sexbell' kernel/src/init.rs` | ≥ 1 |
| Quil still at domain 9 | `grep 'domain_id == 9' kernel/src/init.rs` | Line exists |
| SLOT_QUIL=11 unchanged | `grep 'SLOT_QUIL' crates/sex-pdx/src/lib.rs` | `pub const SLOT_QUIL: u64 = 11;` |
| SLOT_BELL=12 unchanged | `grep 'SLOT_BELL' crates/sex-pdx/src/lib.rs` | `pub const SLOT_BELL: u64 = 12;` |
| Boot log shows Bell | QEMU serial output | `[kernel.spawn.sexbell]` and `[bell.boot]` |
| No fault/panic | Boot log | Clean boot |
| No sexdisplay writes | `grep SLOT_DISPLAY servers/sexbell/src/main.rs` | Empty |
| No sexstore calls | `grep SLOT_SEXSTORE servers/sexbell/src/main.rs` | Empty |

---

## 7. STOP FIRST Gates

**STOP FIRST** before any of the following in the boot spawn implementation phase:

| # | Condition | Why |
|---|-----------|-----|
| S1 | Changing any existing domain/PKEY | Adding Bell domain 10 must not shift existing servers |
| S2 | Removing or reordering `module_paths` entries | Must append only |
| S3 | Granting notify caps to apps | Bell not ready to accept app events |
| S4 | Enabling `OP_BELL_NOTIFY` parsing | Protocol phase not reached |
| S5 | Adding queue/storage/rendering | Server stub only — no behavior |
| S6 | Touching sexdisplay | Renderer boundary not crossed |
| S7 | Touching silk-shell | Shell integration phase not reached |
| S8 | Touching sexstore/sexshop | Storage phase not reached |
| S9 | Changing Quil spawn/order/slot | Only if explicitly justified and audited |
| S10 | Changing ABI/opcodes | Already assigned; no changes |
| S11 | Adding heap allocation | Not needed for stub |
| S12 | Adding any `OP_BELL_*` parsing | Unknown-reject only in stub phase |

### Post-Spawn STOP FIRST

After a successful spawn proof, the next phase must STOP FIRST before:

| # | Condition |
|---|-----------|
| P1 | Sending test messages to Bell from any PD |
| P2 | Parsing any `OP_BELL_*` in listen loop |
| P3 | Adding any cap grants beyond SLOT_BELL self |
| P4 | Merging Bell into any existing test framework |

---

## 8. Negative Checks

| Check | Expected | Rationale |
|-------|----------|-----------|
| No OP_BELL_* parsed in stub | ✅ Only `[bell.unknown.reject]` | Protocol not implemented |
| No title/body transport | ✅ No string fields | Protocol V1 explicitly forbids |
| No app sender identity trusted | ✅ Kernel-authoritative `caller_pd` only | Architecture invariant |
| No event queue allocated | ✅ No ring buffer or list | Stub phase |
| No persistence | ✅ No sexstore calls | Storage phase |
| No renderer policy | ✅ No sexdisplay calls | Rendering phase |

---

## 9. Next Phase

**BELL_BOOT_SPAWN_V1** — The implementation phase. Exact scope:

### Allowed Edits

| File | Edits |
|------|-------|
| `kernel/src/init.rs` | Add `sexbell_id` variable, append to `module_paths`, add domain-10 capture, add SLOT_BELL self-cap grant |
| `docs/handoff/BELL_BOOT_SPAWN_V1.md` | Implementation handoff |

### Explicitly NOT in Scope

- No OP_BELL_* parsing
- No app notify caps
- No display/storage/SilkBar/action caps
- No sexdisplay/silk-shell/sexstore edits
- No Quil changes
- No ABI/opcode changes
- No heap/queue/struct additions

---

## References

- `BELL_NAMESPACE_COLLISION_AUDIT_V1.md` — domain 10, PKEY 10, SLOT_BELL=12
- `BELL_SLOT_OPCODE_ASSIGNMENT_V1.md` — OP_BELL_* 0xC0-0xC7
- `BELL_SERVER_STUB_V1.md` — sexbell crate exists and compiles
- `BELL_SERVER_STUB_PLAN_V1.md` — corrected implementation plan
- `kernel/src/init.rs` — spawn table, domain assignments, cap grants
- `servers/sexbell/src/main.rs` — stub source
- `sexos_build_spec.toml` — build/packaging already configured

---

*End of BELL_BOOT_SPAWN_PLAN_V1.md*
