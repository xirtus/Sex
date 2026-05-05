# E1_STORAGE_BOUNDARY_AUDIT_V1

**Status:** Docs only. No code changed.

**Audit date:** 2026-05-05

**Review gate:** "Accept E1 only if it is docs/audit only and does not start Linen,
Quil persistence, or raw path storage."

---

## Summary

Audit of the current SexOS storage boundary: sexstore (RAM K/V, domain 8),
its clients, slot assignment, key namespace, authority model, and the gap
to the Track E maturity ladder defined in
`docs/PERSISTENT_STORAGE_MATURITY_PLAN_V1.md`.

---

## 1. Current Storage Topology

### 1.1 sexstore server (`servers/sexstore/src/main.rs`)

| Property | Value |
|----------|-------|
| Domain ID | 8 |
| Slot constant | `SLOT_SEXSTORE = 10` (in `crates/sex-pdx/src/lib.rs`) |
| Listen method | `pdx_listen_raw(0)` — self message ring |
| Reply method | `kv_reply()` — inline syscall 29 (`SYSCALL_PDX_REPLY`) |
| Table type | `static mut [KvSlot; 16]` — fixed-size, no heap |
| Slot structure | `KvSlot { used: u8, key: u32, val: u64 }` — 16 bytes |
| Total table size | 256 bytes |
| Lookup algorithm | Linear scan (worst-case 16 iterations) |
| Insert algorithm | Update in-place if key exists; first-free otherwise |
| Full-table behavior | Returns `KV_PUT_FULL (0x02)` — caller drops silently |

### 1.2 Protocol

| Direction | Opcode | arg0 | arg1 | Reply |
|-----------|--------|------|------|-------|
| GET | `OP_KV_GET = 0xB0` | key (u32) | — | stored u64 (0 = miss) |
| PUT | `OP_KV_PUT = 0xB1` | key (u32) | val (u64) | `0x00 = ok`, `0x02 = full` |

Opcodes are **local copies** in both `servers/sexstore/src/main.rs` and
`servers/silk-shell/src/main.rs` — not promoted to `crates/sex-pdx/src/lib.rs`.
This avoids ABI hash churn but duplicates the constants across two files.

### 1.3 Opcode space allocation

Sexstore opcodes (`0xB0–0xB1`) reside in a range that does not collide with
other known protocol ranges:

| Range | Owner |
|-------|-------|
| `0xD0` | Quil ping |
| `0xE4–0xE8` | sexdisplay window ops |
| `0xF0–0xF4` | SilkBar protocol |
| `0xFC` | Appearance tokens |
| `0xFD` | Surface tab info |
| `0x202` | HID events |
| **`0xB0–0xB1`** | **sexstore K/V (assigned)** |

### 1.4 Clients

**Only one client currently uses sexstore:**

| Client | Slot | Opcodes used | Keys used | Purpose |
|--------|------|-------------|-----------|---------|
| `silk-shell` (domain 3) | `SLOT_SEXSTORE = 10` | `OP_KV_GET`, `OP_KV_PUT` | `0x01` (scene appearance) | Boot load + F5 save of `preset_idx`, `chrome_flags`, `accessibility_flags` |

No other domain has been granted `SLOT_SEXSTORE` capability. `kernel/src/init.rs`
grants it only to silk-shell:

```rust
if sexstore_id != 0 {
    pd.grant_capability(sex_pdx::SLOT_SEXSTORE, CapabilityData::Domain(sexstore_id));
    serial_println!("[kernel.sexstore.cap] shell={} store={}", silkshell_id, sexstore_id);
}
```

### 1.5 Capability Grant Topology

```
kernel (domain 0)
  └─ silk-shell (domain 3)
       └─ SLOT_SEXSTORE → sexstore (domain 8)  ← ONLY grantee
```

No other PD has a capability to call sexstore. SexAudio, Theremin, Linen,
Quil, sexfiles have no sexstore path.

---

## 2. Authority Boundary

### 2.1 Who may read/write

| Entity | Read? | Write? | Authority basis |
|--------|-------|--------|----------------|
| silk-shell | ✅ Yes (key `0x01` only) | ✅ Yes (key `0x01` only) | `SLOT_SEXSTORE` cap in kernel init |
| kernel (domain 0) | ✅ Yes (bypasses PDX, owns cap table) | ✅ Yes | Boot-time cap genesis |
| All other PDs | ❌ No | ❌ No | No `SLOT_SEXSTORE` capability granted |

### 2.2 What is forbidden in V1

| Operation | V1 status | Reason |
|-----------|-----------|--------|
| App PDs calling sexstore | ❌ Forbidden | No cap granted; no app PD exists yet |
| Cross-domain key write | ❌ Forbidden | Only silk-shell has write cap |
| sexfiles integration with sexstore | ❌ Not designed | sexfiles has independent VFS backend |
| Disk/persistent backend | ❌ Forbidden | RAM-only V1 per maturity plan |
| Linen storage dependency | ❌ Forbidden | Linen must not depend on sexstore until E2–E8 gates pass |
| Quil persistence | ❌ Forbidden | Quil has no storage path; surfaces are shell-lifecycle-only |
| Raw path/string-based storage | ❌ Forbidden | All storage is `u32 key → u64 value`; no strings, no paths |
| `OP_KV_DEL` (delete/tombstone) | ❌ Not implemented | No delete opcode exists in V1 |
| Key range > single `u64` | ❌ Not supported | V1 value limited to 8 bytes |

### 2.3 Current enforcement mechanism

Access control relies entirely on **capability slot grants** in
`kernel/src/init.rs`. There is no per-key access control, no StoreCapability
check, no caller authentication inside sexstore itself. Any PD granted
`SLOT_SEXSTORE` can read/write any key.

**This is acceptable for V1** because:
- Only one PD (silk-shell) has the capability
- silk-shell is a trusted system component (shell)
- No app PDs exist yet
- E3 (StoreCapability policy) will add per-key access control later

---

## 3. Key Namespace Audit

### 3.1 Currently allocated keys

| Key | Owner | Value | Purpose |
|-----|-------|-------|---------|
| `0x01` | silk-shell | Packed u64: `{ magic: 0xAC, version: 0x01, preset_idx, chrome_flags, accessibility_flags, _reserved, checksum }` | Scene appearance settings |
| `0x00` | (reserved) | — | Invalid key (never stored) |

### 3.2 Value encoding (key `0x01`)

```
Byte 0: magic      = 0xAC
Byte 1: version    = 0x01
Byte 2: preset_idx (0..3)
Byte 3: chrome_flags
Byte 4: accessibility_flags
Byte 5: reserved
Byte 6: reserved
Byte 7: checksum   = XOR(byte0 .. byte6)
```

### 3.3 Key range collision status

No collisions exist because only one key (`0x01`) is allocated.

The maturity plan assigns future ranges:
- `0x10–0x1F`: Theremin settings
- `0x20–0x2F`: Audio policy (SexAudio)
- `0x30–0x3F`: Scene appearance (shell) — **migrate key `0x01` here in E2**
- `0x40–0x4F`: Input config (shell)
- `0x50–0x5F`: Future Linen documents
- `0x60–0x6F`: Future app storage
- `0x70–0x7F`: Admin/debug

### 3.4 Gaps

| Gap | Impact | Planned fix |
|-----|--------|-------------|
| No key range allocation table as single source of truth | Range collisions possible when new clients add keys | E2: define key namespace allocation, range ownership, collision detection |
| Shell key `0x01` outside planned `0x30–0x3F` range | Non-standard allocation | E2: migrate to `0x30`+ range |
| No delete/tombstone opcode | Keys cannot be removed; slot exhaustion permanent | E6: add `OP_KV_DEL` with tombstone semantics |
| No multi-key support | Complex state requires packed u64 blob; 8-byte limit reached quickly | Future: chunked PUT/GET or multi-slot value |
| No key versioning | Schema changes (e.g., new blob format) have no version track | E4: StorageVersion model |

---

## 4. Persistence Backend Status

### 4.1 Current: RAM-only

| Property | Status |
|----------|--------|
| Backing store | `static mut [KvSlot; 16]` — kernel .bss RAM |
| Survival across power cycle | **No** — contents lost on reboot |
| Survival across sexstore restart | **No** — restarted PD reinitializes table to zeros |
| Survival within same boot | **Yes** — RAM persists as long as PD runs |
| Disk/block device integration | **None** — no block device abstraction exists |
| File system backend | **None** — sexfiles is a separate VFS server, not connected to sexstore |
| Write-through cache | **Not applicable** — no disk target |
| Journal/transaction log | **None** — no write-ahead logging |
| Checksum/integrity | **Minimal** — per-blob XOR checksum in key `0x01`, no table-level integrity |

### 4.2 sexfiles status

`servers/sexfiles/src/main.rs` is a Phase 19 trampoline VFS with:
- Multiple backends (not audited here)
- Cache layer
- Message routing

sexfiles is **not integrated** with sexstore. They are separate storage
servers with independent architectures. sexfiles has no sexstore cap.

### 4.3 Invariant summary

> sexstore will remain RAM-only for the entire V1 lifecycle.
> No disk-backed persistence until E9 gate passes.
> All "persistence" across power cycles requires a future block device
> abstraction that has not yet been designed.

---

## 5. Track E Maturity Ladder (from PERSISTENT_STORAGE_MATURITY_PLAN_V1)

The full ladder defined in `docs/PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` §16:

| Phase | Name | Description | Depends on |
|-------|------|-------------|------------|
| **E1** | **STORAGE_BOUNDARY_AUDIT_V1** | ⬅ This document. Audit current storage topology, authority, key namespace, persistence status. No code. | — |
| E2 | KEY_NAMESPACE_RANGE_V1 | Define key namespace allocation, range ownership model, collision detection | E1 |
| E3 | STORAGE_CAPABILITY_POLICY_V1 | Define StoreCapability kinds, caller+op+range validation, Collar integration | E2 |
| E4 | SEQUENCE_SCHEMA_VERSION_V1 | sequence_id model, StorageVersion, migration rules | E3 |
| E5 | CORRUPTION_PARTIAL_FAILURE_V1 | Checksum model, CorruptionState FSM, PartialFailureRecord | E4 |
| E6 | DELETE_TOMBSTONE_V1 | OP_KV_DEL, slab model, tombstone semantics, idempotent delete | E5 |
| E7 | DETERMINISTIC_RAM_STORE_PROOFS_V1 | Structured proof markers for all ops, deterministic behavior proofs | E6 |
| E8 | PRIVACY_REDACTION_PROOF_POLICY_V1 | Redaction classes, persistent log policy. **Must follow E7 directly.** | E7 |
| E9 | PERSISTENT_BACKEND_GATE_V1 | Gate requirements for disk-backed persistence. V1 remains RAM-only. | E8 |
| E10 | SEXFILES_SEXSHOP_INTEGRATION_V1 | sexfiles/sexshop integration with maturity guarantees | E9 |
| E11 | STORAGE_CUSTOMIZATION_POLICY_V1 | Customization/preference model, non-customizable boundaries | E9, E8, E7 |

### 5.1 Current position

E1 completes the audit. The project is at the **start** of the E ladder.
No E2+ phase has been designed or implemented.

### 5.2 V1 scope vs future scope

| Capability | V1 | Future |
|------------|----|--------|
| Key count | 16 slots | Configurable (max 64, E2) |
| Key range allocation | Single key `0x01` | Ranges per PD (E2) |
| Access control | PDX slot only | StoreCapability (E3) |
| Schema versioning | Implicit magic byte | Explicit StorageVersion (E4) |
| Corruption detection | Per-blob XOR checksum | Table CRC + per-entry CRC (E5) |
| Delete | Not available | OP_KV_DEL + tombstone (E6) |
| Proof markers | Budgeted LOG_PUT/LOG_GET | Structured `[store.kv.*]` (E7) |
| Privacy redaction | None | Redaction classes (E8) |
| Disk persistence | None | Gate requirements only (E9) |
| sexfiles integration | None | Integration design (E10) |
| Customization | None | Preference model (E11) |

---

## 6. Proof Marker Audit

### 6.1 Existing markers in sexstore

| Marker | Budget | Location | When |
|--------|--------|----------|------|
| `[sexstore.kv.put] key=N ok=0\|1` | 32 | `main.rs:113` | PUT received |
| `[sexstore.kv.get] key=N hit=0\|1` | 32 | `main.rs:137` | GET received |

Both decrement from 32 to 0 then stop.

### 6.2 Existing markers in silk-shell (sexstore clients)

| Marker | Budget | When |
|--------|--------|------|
| `[shell.scene.settings.load.request] ok=1 pending` | 1 | Boot GET fired |
| `[shell.scene.settings.load.request] ok=0 status=N` | 1 | Boot GET failed |
| `[shell.scene.settings.load] ok=1 preset=N chrome=N access=N` | 1 | Successful load |
| `[shell.scene.settings.load] ok=0 corrupt` | 1 | Blob validation failure |
| `[shell.scene.settings.save] preset=N` | 16 | F5 preset cycle saved |

### 6.3 Gaps (relative to E7 target format)

| Gap | Target (E7) |
|-----|-------------|
| No sequence_id | Every marker has a sequence_id |
| No redaction_class | Every marker has `redact=public\|session\|private\|secure` |
| No delete markers | `[store.kv.del]` when delete opcode exists |
| No corruption markers | `[store.corrupt.detect]` when CRC available |
| No capability deny markers | `[store.capability.deny]` when StoreCapability exists |
| Budgeted (stops at 0) | Persistent proof markers do not stop |

---

## 7. STOP FIRST Conditions

These conditions apply to **any future E phase** (not to this E1 audit):

| Condition | Applies to | Action |
|-----------|-----------|--------|
| Disk-backed persistence before RAM store provably correct | E9+ | STOP — V1 is RAM-only |
| Any storage op without proof marker | E7+ | STOP — every op must have proof marker |
| Persistent log storing Private/Secure content without redaction | E8+ | STOP — redaction policy must exist first |
| Silent schema migration that drops/corrupts data | E4+ | STOP — migration must log proof marker |
| Access control relying on PD identification alone without capability validation | E3+ | STOP — post-E3, PD slot alone is never authority |
| Linen document/project implementation before E2–E8 gates pass | F-track | STOP — Linen depends on mature storage |
| Any unbounded storage growth without eviction/tombstone/reclamation | E6+ | STOP — policy required |
| Any kernel block device ABI changes for storage | E9+ | STOP — no kernel changes |
| Any cross-PD raw pointer storage access | Any | STOP — forbidden |
| Any storage operation that bypasses sexstore/sexfiles and writes directly to hardware | Any | STOP — must go through sexstore/sexfiles |
| Any E1 phase skipped or deferred | E2+ | STOP — maturity targets must be based on audited reality |
| Starting Linen, Quil persistence, or raw path storage | E1 gate | **STOP FIRST — this E1 audit is docs-only; no such work started** |

### 7.1 E1 gate status

> ✅ **E1 passes its own gate.** This document is docs/audit only.
> No Linen, Quil persistence, or raw path storage has been started.

---

## 8. Readiness for E2+

### 8.1 What is ready

- sexstore is spawned, listening, and serving K/V operations
- One client (silk-shell) demonstrates GET/PUT lifecycle
- Capability slot assignment (`SLOT_SEXSTORE = 10`) is stable
- Key namespace (`0x01`) is allocated and in use
- The maturity ladder (`PERSISTENT_STORAGE_MATURITY_PLAN_V1.md`) defines clear E2–E11 phases
- No collisions or conflicts exist in the current storage topology

### 8.2 What blocks E2+

**Nothing blocks E2.** Key namespace range design (E2) is a docs-only phase
that can proceed immediately. E2 does not require code changes.

E3 (StoreCapability policy) is also docs-only but should reference Collar
capability model — this may require understanding Collar's current state
before proceeding.

E4+ require sexstore code changes and are deferred until the design phases
(E2, E3) are complete.

---

## 9. Files Referenced

| File | Relevance |
|------|-----------|
| `servers/sexstore/src/main.rs` | Current sexstore implementation (148 lines) |
| `servers/silk-shell/src/main.rs` | Only client; local opcodes + load/save helpers |
| `crates/sex-pdx/src/lib.rs` | `SLOT_SEXSTORE = 10` constant, `ERR_SERVICE_NOT_READY` |
| `kernel/src/init.rs` | sexstore spawn (domain 8), cap grant to silk-shell |
| `kernel/src/syscalls/store.rs` | `sys_store_fetch()` — unused (package management) |
| `docs/PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` | Track E master plan, ladder, invariants |
| `docs/handoff/SEXSTORE_KV_RAM_V1.md` | sexstore K/V implementation handoff |
| `docs/handoff/SEXSTORE_KV_API_PLAN_V1.md` | API design that KV_RAM implemented |
| `docs/handoff/SEXSTORE_KERNEL_ENABLE_V1.md` | Kernel spawn + cap grant handoff |
| `docs/handoff/SCENE_SETTINGS_STORAGE_PLAN_V1.md` | Storage model, SceneSettingsBlob design |
| `docs/handoff/SCENE_SETTINGS_PERSIST_V1.md` | Silk-shell persistence implementation |

---

## 10. Next Phase: E2_KEY_NAMESPACE_RANGE_V1

Design key namespace allocation:
1. Define range allocation table as single source of truth (migrate key `0x01` to `0x30+` range)
2. Collision detection at plan/compile time
3. Range ownership model (which PD owns which range)
4. Handoff doc only — no code
