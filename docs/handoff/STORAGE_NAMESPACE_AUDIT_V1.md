# STORAGE_NAMESPACE_AUDIT_V1

**Status:** Docs/audit only. No code changed.

**Date:** 2026-05-05

**Review gate:** Resolve sexstore vs sexshop naming/authority confusion. Docs/audit only.

---

## Executive Summary

**No runtime conflict exists.** `sexstore` is the active storage server. `sexshop` is dead placeholder code that is not built, not spawned, and has no PDX slot. The naming confusion is entirely in documentation, where older docs use "n" as the canonical K/V name while recent E-track handoffs use "sexstore".

**Verdict:** Continue E-track (including E13) on `sexstore`. Leave `sexshop` as dead placeholder until a real implementation plan exists.

---

## 1. Current Code Inventory

### servers/sexstore/ (ACTIVE)

| Aspect | Status |
|--------|--------|
| **Directory** | `servers/sexstore/` — exists with `src/main.rs` |
| **Build** | ✅ In `sexos_build_spec.toml` (built by `make`) |
| **Spawn** | ✅ `kernel/src/init.rs:38` — `"sexstore"` in `module_paths[7]` → domain_id=8 |
| **Cap grant** | ✅ `kernel/src/init.rs:100-102` — `grant_capability(SLOT_SEXSTORE, Domain(sexstore_id))` to silk-shell |
| **PDX slot** | `SLOT_SEXSTORE = 10` in `crates/sex-pdx/src/lib.rs:354` |
| **Client** | silk-shell (domain 3) — `OP_KV_GET(0xB0)`, `OP_KV_PUT(0xB1)`, `OP_KV_DEL(0xB2)` |
| **Implementation** | 16-slot static K/V, generation/tombstone, proof markers, dual-page durable backend (E13) |
| **Last E-phase** | E13 — dual-page atomic swap implemented |

**Code imports:**
```rust
use sex_pdx::{pdx_listen_raw, serial_println, SLOT_SEXSTORE};
```

**Opcode namespace:** `0xB0`‑`0xB2` (local to sexstore, not in sex-pdx):
- `OP_KV_GET = 0xB0`
- `OP_KV_PUT = 0xB1`
- `OP_KV_DEL = 0xB2`

**Proof marker prefix:** `[sexstore.*]` (e.g., `[sexstore.put.allow]`, `[sexstore.durable.write]`)

### servers/sexshop/ (DEAD)

| Aspect | Status |
|--------|--------|
| **Directory** | `servers/sexshop/` — exists with `src/main.rs`, `storage.rs`, `pdx.rs`, `trampoline.rs`, `transactions.rs`, `cache.rs` |
| **Build** | ❌ NOT in `sexos_build_spec.toml` — not built by `make` |
| **Spawn** | ❌ NOT in `kernel/src/init.rs` — not spawned |
| **Cap grant** | ❌ No cap grants — not spawned |
| **PDX slot** | ❌ No `SLOT_SEXSHOP` in `crates/sex-pdx/src/lib.rs` |
| **Implementation** | Stub discovery/registry loop + comments referencing `/etc/n.kv`, `/etc/n/obj/`, POSIX paths |
| **Dependencies** | `libsys` (may not exist in current build tree) |

**Key code (main.rs):**
```rust
static mut REGISTRY: [([u8; 32], u32); 16] = [([0; 32], 0); 16];
// Stub discovery service — not a K/V store. Comments reference POSIX paths.
```

**Storage references (storage.rs):**
```rust
// Search /etc/n.kv (Slot 1)
// Append to /etc/n.kv WAL via VfsWrite
// Map hash to path /etc/n/obj/<hash_hex>
```

These are POSIX path assumptions that do not exist in SexOS. The file is comments-only (no runnable no_std K/V code).

---

## 2. Build/Spawn/Capability Inventory

| Server | Built? | Spawned? | PDX Slot | Domain | Clients |
|--------|--------|----------|----------|--------|---------|
| `sexstore` | ✅ `sexos_build_spec.toml` | ✅ `kernel/init.rs:38` index 7 | `SLOT_SEXSTORE=10` | 8 | silk-shell (domain 3) |
| `sexshop` | ❌ Not in spec | ❌ Not in init.rs | ❌ None | ❌ N/A | ❌ N/A |

**No conflict.** Only one storage server is active. No duplicate slot, no duplicate domain, no duplicate opcode namespace.

---

## 3. PDX Slot/Opcode Inventory

### SLOT constants in sex-pdx

| Constant | Value | Server | Status |
|----------|-------|--------|--------|
| `SLOT_STORAGE` | 1 | sexfiles VFS | Active (separate server) |
| `SLOT_SEXSTORE` | 10 | sexstore K/V | ✅ Active |
| `SLOT_SEXSHOP` | — | sexshop | ❌ Not defined |

### Opcode namespace

| Server | Opcode range | Public ABI? | Status |
|--------|-------------|-------------|--------|
| `sexstore` | `0xB0` (GET), `0xB1` (PUT), `0xB2` (DEL) | ❌ Local to sexstore only | ✅ Documented as local |
| `sexshop` | None defined | — | ❌ No ops |

**No opcode namespace collision.** sexstore opcodes are local. sexshop defines no opcodes.

---

## 4. Documentation Contradiction Table

| Document | Refers to active K/V as | Refers to sexshop | Contradiction? |
|----------|------------------------|-------------------|----------------|
| `PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` | **"n"** throughout (e.g., "n is RAM-only", "n uses fixed 16-slot table") | Mentions sexshop in E10 as future integration | ⚠️ Uses "n" not "sexstore" — historical name. Describes current sexstore behavior correctly. |
| `SCENE_PERSISTENCE_PLAN_V1.md` | **"n K/V service (slot 10, n)"** | Not mentioned | ⚠️ Calls it "n" with parenthetical "sexstore" clarification. |
| `THEREMIN_SYSTEM_SOUND_ENGINE_PLAN_V1.md` | **"n K/V"** throughout | Not mentioned | ⚠️ Uses "n" not "sexstore". |
| `SEXAUDIO_HARP_PHASE_PLAN_V1.md` | **"n K/V"** throughout | Not mentioned | ⚠️ Uses "n" not "sexstore". |
| `B_APP_LAUNCH_SESSION_RESTORE_PLAN_V1.md` | **"n (future, G gate)"** | Not mentioned directly | ⚠️ Refers to future "n" for package metadata — this is sexshop territory. |
| `manual_servers.md` | **"n — Object & Package Store"** (path: `servers/n/src/`) | **"n — Legacy Object Store"** (path: `servers/n/src/main.rs`) | ❌ **Major contradiction.** Describes a hypothetical "n" server at `servers/n/` that doesn't exist. Also describes sexshop as "n" and a legacy "n" placeholder. This doc references paths and servers that don't match the current tree. |
| `E-handoffs (E1–E13)` | **"sexstore"** consistently | Not mentioned | ✅ Consistent — all E-track docs use "sexstore". |
| `kernel/src/init.rs` | **"sexstore"** (spawn name) | Not mentioned | ✅ Correct. |
| `crates/sex-pdx/src/lib.rs` | **`SLOT_SEXSTORE = 10`** | Not mentioned | ✅ Correct. |

### Key contradictions

1. **"n" vs "sexstore":** The master plan and audio/Theremin docs use "n" as the canonical name. All E-track handoffs (E1–E13) use "sexstore". The kernel spawns it as "sexstore". The PDX slot is `SLOT_SEXSTORE`. The code directory is `servers/sexstore/`. **"n" is a historical name that no longer appears in code.**

2. **"sexshop" vs "n":** `manual_servers.md` describes "n" as an object/package store at `servers/n/` — but this server doesn't exist. `servers/sexshop/` is the closest match, but it's dead placeholder code with POSIX path assumptions. The manual is describing a hypothetical server that was never built.

3. **sexshop's POSIX paths:** `servers/sexshop/src/storage.rs` references `/etc/n.kv`, `/etc/n/obj/`, VfsWrite — paths and APIs that don't exist in the no_std SexOS kernel.

---

## 5. Recommended Canon

### Option A: Status quo (RECOMMENDED)

| Role | Server | Status |
|------|--------|--------|
| **System settings K/V** (current) | `sexstore` (domain 8, slot 10) | ✅ Active — all E-track work |
| **Future object/package store** | `sexshop` | ⬜ Dead placeholder — no build, no spawn, no slot |

**Rules:**
- E-track (E4–E13) continues on `sexstore` — it is the canonical system-settings K/V
- `sexshop` stays as dead placeholder code — NOT built, NOT spawned
- No renaming or consolidation until `sexshop` has a real no_std implementation plan
- `sexstore` durable backend (E13) is documented as **system-settings durability only** — not object/package store

### Option B: Rename sexstore → n

Would require:
- Rename directory `servers/sexstore/` → `servers/n/`
- Update `sexos_build_spec.toml`
- Update `kernel/src/init.rs` spawn name
- Update E-track handoffs (all use "sexstore")
- Update proof marker prefixes `[sexstore.*]` → `[n.*]`
- NOT recommended — cosmetic rename with no functional benefit, breaks all existing documentation

### Option C: Rename sexshop → sexstore-object

Would require:
- Rename directory (cosmetic only, not built)
- Add `SLOT_SEXSHOP` to sex-pdx (premature — no implementation exists)
- NOT recommended — sexshop has no implementation to rename

**Recommendation: Option A — status quo.**

---

## 6. E13 Decision

**E13 may continue on sexstore.** 

Rationale:
1. sexstore is the active, built, spawned, production storage server
2. sexshop is dead placeholder code — not built, not spawned, no PDX slot
3. No runtime conflict exists
4. E13's dual-page backend is documented as **system-settings durability** — not object/package store
5. When sexshop is eventually implemented, it will get its own PDX slot, opcode namespace, and E-track phases

**Constraint:** E13 handoff must explicitly state that the durable backend is for sexstore (system settings K/V) only. Future object/package storage (sexshop) requires separate design.

---

## 7. Docs to Update

| Document | Correction needed | Priority |
|----------|------------------|----------|
| `PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` | Replace "n" with "sexstore" throughout — or add a preamble: "sexstore (historically referred to as 'n')" | MEDIUM — misleading but no code impact |
| `THEREMIN_SYSTEM_SOUND_ENGINE_PLAN_V1.md` | Replace "n K/V" with "sexstore K/V" | LOW — future spec, not implemented |
| `SEXAUDIO_HARP_PHASE_PLAN_V1.md` | Replace "n K/V" with "sexstore K/V" | LOW — future spec |
| `SCENE_PERSISTENCE_PLAN_V1.md` | Already has "(slot 10, sexstore)" — clarify that "n" is historical | LOW |
| `manual_servers.md` | Major rework needed — describes servers that don't exist (n/, n-gui/) | HIGH — actively misleading |
| `B_APP_LAUNCH_SESSION_RESTORE_PLAN_V1.md` | Replace "n" with "sexstore" for current, "sexshop" for future object store | LOW — future spec |

**Docs that are correct:**
- All E-track handoffs (E1–E13) — consistently use "sexstore" ✅
- `kernel/src/init.rs` — uses "sexstore" ✅
- `crates/sex-pdx/src/lib.rs` — uses `SLOT_SEXSTORE` ✅

---

## 8. STOP FIRST Findings

| # | Condition | Check | Stop? |
|---|-----------|-------|-------|
| S1 | Both servers spawned with same slot/name | sexstore is spawned. sexshop is NOT spawned. | ❌ Not triggered |
| S2 | Both claim same PDX opcode namespace | sexstore uses `0xB0`-`0xB2` (local). sexshop defines no ops. | ❌ Not triggered |
| S3 | Docs require sexshop now but code only supports sexstore | No doc requires sexshop for current functionality. Future plans mention it but don't block E-track. | ❌ Not triggered |
| S4 | E13 would implement durable backend in wrong server | E13 is in sexstore — the correct server for system-settings durability. | ❌ Not triggered |
| S5 | Renaming requires kernel/sex-pdx edits | Option A (recommended) requires no renames. | ❌ Not triggered |
| S6 | sexshop's POSIX paths conflict with no_std SexOS | sexshop is dead code — not built, not spawned. Path references are in comments only. | ❌ Documented — no action |

**All STOP FIRST conditions pass. No runtime conflict exists.**

---

## 9. Final Verdict

```
STORAGE_NAMESPACE_AUDIT_V1

Status: PASS — no contradiction at runtime.

sexstore role:
  ✅ Current bounded shell/system settings K/V
  ✅ Active E-track server (E4–E13)
  ✅ Built, spawned, capability-gated
  ✅ Only silk-shell (domain 3) has access
  ✅ E13 durable backend is system-settings durability

sexshop role:
  ❌ Dead placeholder code — not built, not spawned
  ❌ POSIX path assumptions incompatible with no_std SexOS
  ❌ No SLOT_SEXSHOP in sex-pdx
  ⬜ Future object/package store — requires real implementation plan

Conflicts:
  ❌ No runtime conflict
  ❌ No slot/opcode collision
  ❌ No duplicate domain

Recommended:
  Option A — status quo. Continue E-track on sexstore.
  No renames. No consolidation.

Docs requiring correction:
  - manual_servers.md (HIGH — describes non-existent servers)
  - PERSISTENT_STORAGE_MATURITY_PLAN_V1.md (MEDIUM — uses "n" for sexstore)
  - Various future-plan docs (LOW — use sexstore instead of "n")
```

---

## Appendix A: Files Referenced

| File | Relevance |
|------|-----------|
| `servers/sexstore/src/main.rs` | Active K/V server — all E4-E13 code |
| `servers/sexshop/src/main.rs` | Dead placeholder — stub discovery loop |
| `servers/sexshop/src/storage.rs` | Dead placeholder — POSIX path comments |
| `servers/sexshop/src/pdx.rs` | Dead placeholder — stub PDX handlers |
| `servers/sexshop/src/transactions.rs` | Dead placeholder — VFS path comments |
| `crates/sex-pdx/src/lib.rs` | `SLOT_SEXSTORE = 10` — no `SLOT_SEXSHOP` |
| `kernel/src/init.rs` | Spawns "sexstore" at domain 8, grants SLOT_SEXSTORE to shell |
| `sexos_build_spec.toml` | Builds sexstore — does NOT build sexshop |
| `docs/PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` | Uses "n" for sexstore throughout |
| `docs/manual_servers.md` | Describes non-existent servers, major contradictions |
| `docs/handoff/E1_STORAGE_BOUNDARY_AUDIT_V1.md` | Uses "sexstore" consistently ✅ |
| `docs/handoff/E4_STORAGE_SCHEMA_VALIDATION_V1.md` through E13 | All use "sexstore" consistently ✅ |

---

*End of STORAGE_NAMESPACE_AUDIT_V1.md*
