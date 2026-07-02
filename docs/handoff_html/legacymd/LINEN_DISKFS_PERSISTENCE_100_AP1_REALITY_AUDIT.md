# LINEN_DISKFS_PERSISTENCE_100_AP1_REALITY_AUDIT

**Date:** 2026-05-24
**Phase:** AP1 — Audit (DOC ONLY, no implementation)
**Track:** Linen → SexFiles → DiskFS user-facing persistence ladder 100
**Predecessor:** sexfiles-diskfs-100-current-tier-v1 (closed, tagged)
**Status:** AUDIT COMPLETE — ladder proposed, no code changes

---

## 1. BASELINE STATUS

| Item | Value |
|------|-------|
| HEAD commit | `025f7515 docs: close SexFiles DiskFS current tier` |
| SexFiles DiskFS tag | `sexfiles-diskfs-100-current-tier-v1` — PRESENT |
| SexDrive tag | `sexdrive-storage-100-current-tier-v1` — PRESENT |
| Working tree | Clean |
| SexFiles AP7 closeout | Committed + tagged per tag presence on HEAD |

---

## 2. FILES INSPECTED

| File | Section | Relevance |
|------|---------|-----------|
| `servers/linen/src/main.rs` | Full (2181 lines) | Linen server entry, all opcode handlers, DiskFS direct+slot proofs, SexFiles metadata bridge |
| `servers/linen/src/session.rs` | Full (227 lines) | LinenObject struct, Session manager, persisted/sexfiles_object_id binding |
| `servers/linen/src/sexobject.rs` | Full (85 lines) | SexObjectHeader/SexObjectRef mapping from LinenObject, OQ5 resolution |
| `servers/linen/Cargo.toml` | Full | Dependencies: sex-pdx, sex-object-model |
| `servers/quil/src/main.rs` | Save/Load + DiskFS slot proof sections | Quil RamFS save/load, DiskFS slot proof at path_id=2 |
| `servers/silk-shell/src/main.rs` | Linen references | Linen keyboard route, focus, object detail/open intent |
| `scripts/daily_driver_master_gate.sh` | Linen+DiskFS gates (lines 44-131, 565-620, 870-883, 3782-3857, 5334, 5626-5630) | All Linen gate definitions and classification logic |
| `scripts/gate_0_2.sh` | Full | No Linen persistence claims found |
| `scripts/gate_no_alpha.sh` | Full | No Linen persistence claims found |
| `scripts/gate_render.sh` | Full | No false Linen persistence claims |
| `docs/handoff/FINAL_LINEN_DISKFS_BRIDGE_AUDIT_V1.md` | Full | Prior Linen→DiskFS bridge audit (2026-05-07) |
| `docs/handoff/LINEN_DISKFS_DIRECT_OBJECT_PROOF_V1.md` | Full | Linen direct DiskFS proof (128B roundtrip) |
| `docs/handoff/LINEN_DISKFS_SLOT_OBJECT_PROOF_V1.md` | Full | Linen V2 slot proof at path_id=1 (16B) |
| `docs/handoff/LINEN_OBJECT_PERSIST_ASYNC_V1.md` | Full | Async fire-and-forget CREATE_OWNER proof |
| `docs/handoff/LINEN_PERSIST_READBACK_MODEL_V1.md` | Full | Persist state model (durable=0, sync_readback=0) |
| `docs/handoff/SEXFILES_LINEN_OBJECT_METADATA_PERSISTENCE_V1.md` | Full | Linen→SexFiles metadata bridge with PDX proxy blocker |
| `docs/handoff/SEXFILES_DISKFS_100_AP1_REALITY_AUDIT.md` | Sections 1-7 | SexFiles DiskFS bridge audit (2026-05-22) |
| `docs/handoff/LINEN_SEXFILES_100_AUTOPILOT_2_DISKFS_FIXED_OBJECT_BRIDGE.md` | Full | CASE 2 honest blocker classification for DiskFS bridge |
| `docs/handoff/QUIL_DISKFS_SLOT_OBJECT_PROOF_V1.md` | Full | Quil DiskFS slot proof at path_id=2 |
| `crates/sex-object-model/src/lib.rs` | Full | SexObjectKind, SexObjectRef, SexObjectHeader definitions |

---

## 3. PHASE C — CURRENT SOURCE REALITY ANSWERS

### Q1: Does Linen currently write object data to SexFiles?

**YES, but gate-dependent.** Linen has three storage write paths:

| Path | Function | Gate | Writes to | Real data? |
|------|----------|------|-----------|------------|
| Metadata persist | `linen_persist_object()` | `SEXOS_LINEN_SEXFILES100_PROOF` | RamFS via OP_RAMFS_* | Yes — 48B metadata record |
| Direct DiskFS proof | `run_linen_diskfs_direct_proof()` | `SEXOS_LINEN_DISKFS_DIRECT_PROOF` | DiskFS via OP_DISKFS_* | Yes — 128B deterministic payload |
| V2 slot proof | `run_linen_diskfs_slot_proof()` | `cfg!(linen_diskfs_slot_proof)` | DiskFS via OP_DISKFS_SELECT+WRITE | Yes — 16B deterministic payload |
| Async persist | `run_linen_object_persist_proof()` | `SEXOS_LINEN_OBJECT_PERSIST_PROOF` | RamFS CREATE_OWNER only (no WRITE) | Partial — metadata file name only |
| Disk object proof | `run_linen_disk_object_proof()` | `SEXOS_LINEN_DISK_OBJECT_PROOF` | RamFS via OP_RAMFS_* | Yes — 128B payload |

**None of these paths run on default boot.** All are gated behind compile-time env vars that default to OFF.

### Q2: Does Linen currently read object data from SexFiles?

**YES, but gate-dependent and proof-only.** Read paths:
- `linen_readback_verify()` — reopens RamFS meta-file by name, reads filename bytes via OP_RAMFS_READNAME, verifies match. Behind `SEXOS_LINEN_SEXFILES100_PROOF`.
- `run_linen_diskfs_direct_proof()` — reads 128B back via OP_DISKFS_READ (16 chunks × 8B), verifies byte-for-byte match. Behind `SEXOS_LINEN_DISKFS_DIRECT_PROOF`.
- `run_linen_diskfs_slot_proof()` — reads 16B back via OP_DISKFS_READ (2 chunks × 8B), verifies match. Behind `cfg!(linen_diskfs_slot_proof)`.
- `run_linen_disk_object_proof()` — reads 128B back from RamFS via OP_RAMFS_READ. Behind `SEXOS_LINEN_DISK_OBJECT_PROOF`.

**No synchronous readback on default boot.** All require explicit proof env vars.

### Q3: Does Linen persist object metadata: id/kind/name/state?

**YES, behind gate.** `linen_persist_object()` packs a 48-byte metadata record:
```
bytes 0-7:   object_id (u64 LE)
bytes 8-9:   kind (u16 LE)
bytes 10-13: owner_pd (u32 LE)
bytes 14-21: generation (u64 LE)
bytes 22:     flags (u8)
bytes 23:     name_len (u8)
bytes 24-47: name (24 bytes)
```
Written as 6 × 8B chunks via OP_RAMFS_WRITE to a RamFS file named `lo.{object_id:016x}`.

Session model tracks `flags` bit 0 (`persisted`) and binds the RamFS handle and SexFiles global object_id.

**State persist is honest about limits:**
- `[linen.persist.truth] durable=0 sync_readback=0` — explicitly not durable
- `[linen.persist.state] state=new/dirty/persist_sent/status_requested/status_known` — 5-state model

### Q4: Does Linen persist object content?

**NO.** Linen's object model (`LinenObject` in `session.rs`) has NO content field. The struct only stores metadata: id, kind, owner_pd, name, ramfs_handle, sexfiles_object_id, generation, flags. There is no payload/content buffer in the Linen object model.

The `handle_create_object` handler persists METADATA only via `linen_persist_object()`. Object content would need to be stored separately — Quil owns text content, not Linen.

### Q5: Does Linen have save/load opcodes?

**NO.** Linen's PDX opcodes are:
- `OP_LINEN_CREATE_OBJECT` (0x41) — create with kind+name
- `OP_LINEN_LIST_OBJECTS` (0x42) — list owned objects
- `OP_LINEN_GET_OBJECT` (0x43) — get object info
- `OP_LINEN_GET_PUBLIC_SNAPSHOT` (0x44) — public slot read
- `OP_LINEN_GET_PUBLIC_NAME` (0x45) — public name read
- `OP_LINEN_OPEN_INTENT` (0x46) — stub open intent
- `OP_LINEN_SEARCH_OBJECTS` (0x47) — search by token

**No OP_LINEN_SAVE / OP_LINEN_LOAD opcodes exist.** Persistence is attempted at create time via `linen_persist_object()` inline in `handle_create_object`, not via user-requestable opcodes.

### Q6: Does Linen use DiskFS directly, SexFiles indirectly, or not at all?

**ALL THREE, depending on gate configuration:**

| Gate | Route |
|------|-------|
| None (default) | Not at all — no persistence attempted |
| `SEXOS_LINEN_SEXFILES100_PROOF` | SexFiles RamFS (indirect) via OP_RAMFS_* (0x30-0x37) |
| `SEXOS_LINEN_DISK_OBJECT_PROOF` | SexFiles RamFS (indirect) via OP_RAMFS_* (0x30-0x35) |
| `SEXOS_LINEN_DISKFS_DIRECT_PROOF` | DiskFS (direct bridge) via OP_DISKFS_* (0x38-0x3C) |
| `cfg!(linen_diskfs_slot_proof)` | DiskFS V2 (slot) via OP_DISKFS_SELECT (0x3E) + OP_DISKFS_* |

**Full route for DiskFS direct:**
```
Linen (PD 7) → SLOT_STORAGE(1) → SexFiles VFS (PD 11) → DiskFS → SLOT_BLOCK(15) → SexDrive (PD 2) → NVMe
```

### Q7: Does Quil edit content that Linen can persist?

**NO direct path.** Quil edits text content in its own static buffer (`QUIL_BUFFER`). Quil saves to RamFS via `quil_save()` → `OP_RAMFS_WRITE` to file `quil_doc_01`. This is a Quil-owned RamFS file — Linen does not read or reference it.

Quil also has a DiskFS slot proof (`run_quil_diskfs_slot_min_proof()`) at path_id=2 (`/disk/quil-object-v1`), but this is a proof-only 16B pattern, not Quil's text buffer content.

**There is no Linen→Quil save path and no Quil→Linen content return path.** The J4/J7 Linen→Quil object links exist in the Silk Shell UI (selecting a Linen object opens Quil), but there is no content persistence bridge between them.

### Q8: Is there an existing object ID → DiskFS path mapping?

**YES, proof-only.** Linen maps its objects to:
- RamFS names: `lo.{object_id:016x}` (18 bytes, via `make_linen_meta_name()`)
- DiskFS V2 slot path: `path_id=1` → `/disk/linen-object-v1` (LBAs 2030-2037 per manifest)

The V2 multi-object manifest (up to 3 paths at LBAs 2030-2045) is defined in `vfs.rs` and `diskfs.rs` but Linen only uses path_id=1. The manifest is bootstrapped lazily by `diskfs_ensure_manifest_v2()`.

**This is NOT a general object_id → path mapping.** It's a fixed-slot proof mapping. Real user objects would need a dynamic id→name→path→LBA chain that doesn't exist yet.

### Q9: Are delete/rename/folder semantics real or stubbed?

**ALL STUBBED.** Evidence from `servers/linen/src/main.rs`:

- **Delete:** `linen_nav_delete_current_safe()` (line 239-246) prints `"[linen.delete.proof] ok=0 reason=no_safe_reversible_delete_path"`. Always returns false.
- **Rename:** No opcode, no handler, no path. ABSENT.
- **Folder/path:** Explicit non-claim in session.rs docstring: "No POSIX paths, no filesystem semantics." Linen object names are display labels only.

### Q10: Are any gates falsely claiming Linen persistence?

**NO.** All Linen persistence gates correctly default to SKIP and only PASS when specific proof markers are present:

| Gate | Default | PASS condition | Honest? |
|------|---------|---------------|---------|
| `linen_object_persist` | SKIP | `linen.object.persist.audit` or `linen.object.persist.proof.done ok=1` | YES — says "persist audit present (partial)" |
| `linen_persist_readback` | SKIP | `linen.persist.readback.proof.done ok=1` or `linen.persist.truth` | YES — says "persist model (durable=0 sync=0)" |
| `linen_sexfiles100_audit` | SKIP | `linen.sexfiles100.audit.done ok=1` | YES |
| `linen_objects_list` | SKIP | `linen.objects.list.done` present | YES |
| `linen_ramfs_crud` | SKIP | `linen.ramfs.read.match ok=1` | YES |
| `linen_diskfs_direct` | SKIP | Full 128B write/read/match roundtrip | YES — explicit dectection of fake match + honest Skip for CASE 2 blocker |

### Q11: What is the smallest safe Linen object to persist first?

**A single fixed Linen object with bounded metadata + content through SexFiles RamFS, then through DiskFS.**

The `linen_diskfs_direct_proof()` already demonstrates the shape: one fixed object (id `0x3156_4E45_4E49_4C` = "LINEN_V1"), 128 bytes total (48 bytes metadata + 80 bytes content guard), written through DiskFS, read back, byte-matched.

The smallest safe AP2 target:
- One Linen object ID (deterministic, e.g., object_id=1 from `linen_init_session`)
- Bounded metadata payload (48 bytes, existing format)
- Bounded content payload (up to 128 bytes)
- Save through SexFiles RamFS first, then DiskFS
- Load through SexFiles RamFS first, then DiskFS
- Byte-for-byte match verification
- No Quil, no folders, no delete, no rename

### Q12: What is the smallest safe proof path for AP2?

**Linen fixed-object save/load through SexFiles DiskFS, proved at RamFS layer with existing Opcodes then bridged to DiskFS.**

Exact path:
```
1. Linen creates object (SESSION.create) → local object_id=1
2. Linen saves object via existing OP_RAMFS_* opcodes → SexFiles RamFS
3. Linen reads back from SexFiles RamFS → byte match (AP2a: RamFS roundtrip)
4. Linen writes same payload via OP_DISKFS_* opcodes → SexFiles DiskFS → NVMe
5. Linen reads back from SexFiles DiskFS → byte match (AP2b: DiskFS roundtrip)
```

The existing proofs demonstrate each piece independently (RamFS roundtrip via `linen_disk_object_proof`, DiskFS roundtrip via `linen_diskfs_direct_proof`), but they use synthetic/deterministic objects, not real user-object-id-based save/load.

---

## 4. PHASE D — CLASSIFICATION TABLE

| Component | Status | Evidence Source/Marker |
|-----------|--------|----------------------|
| Linen object list source | **PROVEN** | `session.rs` — bounded 16-slot static table, `SESSION.count()`, `SESSION.list()` |
| Linen seeded objects | **PROVEN** | `linen_init_session()` — 5 fixed boot entries (SexOS Kernel, Silk Shell, etc.) |
| Linen object metadata persistence | **PROVEN (gate-dependent)** | `linen_persist_object()` — 48B record to RamFS; behind `SEXOS_LINEN_SEXFILES100_PROOF` |
| Linen object content persistence | **ABSENT** | LinenObject has no content field; no save/load opcodes |
| Linen save opcode | **ABSENT** | Only CREATE(0x41), LIST(0x42), GET(0x43) exist; no SAVE/LOAD |
| Linen load opcode | **ABSENT** | No opcode to request object data reload from storage |
| Linen → SexFiles call path | **PROVEN** | `pdx_call(SLOT_STORAGE, ...)` — all RamFS and DiskFS opcodes work |
| SexFiles → DiskFS path | **PROVEN** | `SLOT_BLOCK(15)` → SexDrive NVMe; manifest bootstrap, read/write verified |
| Quil edited content save path | **PARTIAL: RamFS only** | `quil_save()` → `OP_RAMFS_WRITE` to `quil_doc_01`; NO DiskFS durability |
| Quil DiskFS slot proof | **PROVEN (proof-only)** | `run_quil_diskfs_slot_min_proof()` at path_id=2; 16B deterministic pattern |
| Reboot restore path for Linen objects | **ABSENT** | No boot-time RamFS/DiskFS readback of user objects into SESSION |
| Delete path | **STUB** | `linen_nav_delete_current_safe()` — "no_safe_reversible_delete_path" |
| Rename path | **ABSENT** | No opcode, no handler |
| Folder/path semantics | **ABSENT** | Explicit non-claim: "No POSIX paths, no filesystem semantics" |
| Negative tests (missing object) | **PROVEN** | `linen_object_detail(0xFFFF)` → not_found_graceful; bounds tests in proofs |
| Negative tests (mismatch) | **PROVEN** | `linen_diskfs_direct` — byte mismatch detection with offset reporting |
| Negative tests (bounds) | **PROVEN** | DiskFS read/write past 4096 in direct+slot proofs; RamFS read past end in disk_object proof |
| Flush/fsync durability | **HONEST NON-CLAIM** | `ERR_NO_DEVICE` on QEMU; no false claim |
| Default gate hygiene | **GOOD** | All persistence gates SKIP by default; no false PASS without explicit env vars |

---

## 5. PHASE E — FALSE CLAIM / GATE RISK AUDIT

### No false claims found. All gates are defensively classified.

**Gate-by-gate audit:**

| Gate | Default | Risk | Notes |
|------|---------|------|-------|
| `linen_nonblocking` | SKIP→PASS with evidence | LOW | Claims nonblocking open only, not persistence |
| `linen_detail` | SKIP→PASS with evidence | LOW | Claims object seeding/detail, not persistence |
| `linen_object_workflow` | SKIP | NONE | Create/tag/search flow, local only |
| `linen_object_persist` | SKIP→PASS with evidence | LOW | Gate description says "persist audit present (partial)" — honest |
| `linen_object_schema` | SKIP | NONE | Kind/status taxonomy, not persistence |
| `palette_linen_available` | SKIP→PASS with evidence | LOW | UI palette status only |
| `linen_search_bridge` | SKIP | NONE | Local-only search |
| `linen_persist_readback` | SKIP→PASS with evidence | LOW | Gate description says "persist model (durable=0 sync=0)" — honest |
| `linen_sexfiles100_audit` | SKIP | NONE | Requires explicit `SEXOS_LINEN_SEXFILES100_PROOF` |
| `linen_objects_list` | SKIP | NONE | Requires `SEXOS_LINEN_SEXFILES100_PROOF` |
| `linen_ramfs_crud` | SKIP | NONE | Requires `SEXOS_LINEN_SEXFILES100_PROOF` |
| `linen_diskfs_direct` | SKIP | LOW | Detects fake match, honest blocker, violation; only PASS on real 128B roundtrip |
| `sexfiles_diskfs_bridge` | SKIP | LOW | Same defense pattern as linen_diskfs_direct |

**No false claims to patch.** Gate hygiene is exemplary for AP1 baseline.

### Potential future risk (not current):
If someone adds a gate that claims `linen_persistence` PASS based solely on RAMFS CREATE_OWNER success (fire-and-forget, no write, no readback), that would be a false claim. The current `linen_object_persist` gate is honest about this ("partial").

---

## 6. PHASE F — PROPOSED LINEN 100 LADDER

### AP1: Reality Audit (THIS DOCUMENT) ✓

### AP2: Linen Fixed-Object Save/Load Through SexFiles DiskFS

**Target:** Prove one Linen object can save its metadata+content through SexFiles RamFS, then through DiskFS, and load it back with byte-accurate verification.

**Prerequisites:** None beyond current AP1 baseline.

**Implementation plan:**
1. Add `OP_LINEN_SAVE_OBJECT` (e.g., 0x48) and `OP_LINEN_LOAD_OBJECT` (e.g., 0x49) opcodes to Linen.
2. `OP_LINEN_SAVE_OBJECT`: arg0=object_id. Linen looks up object in SESSION, packs metadata+content into deterministic payload (up to 128 bytes), writes through existing OP_RAMFS_WRITE path to SexFiles RamFS, then through OP_DISKFS_WRITE path to DiskFS.
3. `OP_LINEN_LOAD_OBJECT`: arg0=object_id. Linen reads back metadata+content from SexFiles DiskFS (fallback to RamFS), verifies byte match, reconstructs LinenObject in SESSION.
4. No Quil involvement yet.
5. No folders/directories.
6. No POSIX claim.
7. Bounded payload (128 bytes max).
8. Byte match verification with explicit marker.
9. Negative: load missing object, load mismatched object, save to full table.

**Safety:** No kernel edits. No sex-pdx ABI edits (new Linen opcodes within Linen's own opcode space). No DiskFS semantic edits. No apps/sexdrive edits.

### AP3: Linen Reboot Restore

**Target:** Save an object, reboot (preserve nvme.img), verify object is restored into session on next boot.

**Implementation plan:**
1. At save time, write object record to known DiskFS manifest slot.
2. At boot (`linen_init_session`), detect persisted object marker in DiskFS manifest.
3. Read back object metadata from DiskFS, reconstruct LinenObject in SESSION.
4. Verify object metadata + content byte match after reboot.
5. Only for bounded known objects (not arbitrary user objects yet).

### AP4: Quil Edit → Linen Save → DiskFS Readback

**Target:** Text content edited in Quil is saved by Linen through DiskFS, then read back.

**Prerequisites:** AP2 working. Quil content path audit required before implementation.
**STOP FIRST if:** Quil source reality does not support Linen-mediated content save path. Current reality: Quil saves to its own RamFS file (`quil_doc_01`). Linen does not read Quil content. A design doc (AP4a) must precede implementation (AP4b).

### AP5: Linen Negative Classifications

**Target:** Classify expected failure modes for object persistence.
1. Missing object → error, not panic.
2. Mismatch → error with offset reporting.
3. Read-only/no-write → honest rejection.
4. Full table → error, not overflow.
5. Invalid object_id → error.
6. Flush/fsync → honest ERR_NO_DEVICE.
7. Permission denied (non-owner) → error.

### AP6: Linen UI Object List Restore Proof

**Target:** After reboot, the restored object appears in the Linen UI object list with correct name/kind metadata. Content marker visible (e.g., via Quil open).

**No general folder semantics.** No arbitrary directory tree.

### AP7: Closeout/Tag

**Tag:** `linen-diskfs-persistence-100-current-tier-v1`
- All AP proofs committed
- Tag applied to closeout commit
- Working tree clean

### Future Tiers (NOT IN SCOPE for 100-track)

- Folders/directories — Linen V2
- Arbitrary path semantics — Linen V2
- Delete/rename/truncate — Object lifecycle track
- Concurrent edits/locking — Multi-PD Quil track
- Journaling/crash consistency — DiskFS V2
- Power-loss durability — SexDrive NVMe flush track

---

## 7. EXPLICIT NON-CLAIMS

The following are explicitly NOT claimed by the LINEN_DISKFS_PERSISTENCE_100 track:

- ❌ POSIX filesystem semantics
- ❌ Arbitrary directories / folder hierarchy
- ❌ Power-loss durability
- ❌ Crash consistency
- ❌ Journaling
- ❌ True fsync/flush to NVMe media
- ❌ Concurrent multi-PD locking
- ❌ Delete / rename / truncate
- ❌ General object_id → DiskFS dynamic path mapping
- ❌ Quil content → Linen persistence (AP4 not yet designed)
- ❌ Reboot restore of arbitrary user objects (AP3 targets bounded known objects first)

---

## 8. STOP FIRST BLOCKERS

| Condition | Status | Action |
|-----------|--------|--------|
| Implementing Linen persistence requires kernel edit | NOT YET KNOWN | AP2 design must verify no kernel edit needed |
| Implementing Linen persistence requires sex-pdx ABI edit | NOT YET KNOWN | New Linen opcodes (0x48, 0x49) are within Linen's existing opcode space; no ABI change |
| Current source reality contradicts expected Linen/SexFiles path | NO | Source confirms the path works (behind gates) |
| Existing gate falsely reports Linen persistence | NO | All gates are honest |
| Tracked tree dirty | NO | Working tree clean |
| sexfiles-diskfs-100-current-tier-v1 tag missing | NO | Tag present on HEAD |
| Uncommitted AP7 closeout files | NO | No uncommitted changes |

---

## 9. FILES CHANGED

None (audit only). One new handoff doc created: `docs/handoff/LINEN_DISKFS_PERSISTENCE_100_AP1_REALITY_AUDIT.md`.

