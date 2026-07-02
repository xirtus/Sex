# E11_DURABLE_BACKEND_DESIGN_V1

**Status:** Docs/spec only. No code changed.

**Date:** 2026-05-05

**Review gate:** "Accept E11 only if it is docs-only and recommends a backend design without implementing persistence or assuming POSIX/filesystem paths."

**Depends on:** E10_MEDIUM_RISK_CLEANUP_V1 (applied), E9_STORAGE_DURABLE_BACKEND_GATE_V1 (gate criteria), E8_STORAGE_REDACTION_POLICY_V1 (redaction classes), E6_STORAGE_TOMBSTONE_DELETE_V1 (slot model), E4_STORAGE_VALUE_VALIDATION_V1 (value envelope)

---

## Table of Contents

1. Design Goals
2. Backend Ownership Model
3. Storage Unit Model
4. Candidate Comparison
5. Recommended Backend: Dual-Page Atomic Swap
6. Atomicity and Recovery Model
7. Privacy and Redaction Model
8. Proof Markers
9. Migration Plan (E12 → E13)
10. Remaining Risks
11. STOP FIRST Conditions

---

## 1. Design Goals

### Goals (must satisfy all)

| # | Goal | Rationale |
|---|------|-----------|
| G1 | **RAM-only semantics preserved as primary storage** | Durable backend is a secondary persistence layer. All protocol operations (PUT/GET/DEL) remain RAM-first. Durable write is synchronous after RAM commit. Boot loads FROM durable INTO RAM. |
| G2 | **No kernel/ABI changes** | Durable backend lives entirely inside sexstore (domain 8). No syscall changes, no PDX message format changes, no capability topology changes. |
| G3 | **No POSIX/filesystem paths** | SexOS is a no_std microkernel. No block device driver, no filesystem, no paths. Durable storage is raw page I/O via existing kernel abstraction. |
| G4 | **Crash-atomic per operation** | A power loss during a single PUT/GET/DEL must not corrupt the durable store. After reboot, the store must reflect exactly the last completed RAM operation — or be recoverable to the last consistent state. |
| G5 | **Privacy/redaction policy inherited** | E8 redaction classes apply to all durable proof markers. No stored values, paths, titles, or user content in any durable log. |
| G6 | **Bounded storage footprint** | Durable storage must use a fixed, pre-allocated region. No dynamic growth, no heap, no allocator dependency. |
| G7 | **Boot-time verification** | On startup, sexstore must verify durable store integrity before loading into RAM. Corrupted records must be detected and handled (default, warn, or skip). |
| G8 | **Minimal code footprint** | Backend implementation target: ~100-150 lines of no_std Rust. No external crates beyond what sexstore already uses. |

### Non-goals (explicitly out of scope)

| # | Non-goal | Why |
|---|----------|-----|
| N1 | Block-level wear leveling | Not needed for RAM/Flash emulation. If real block storage is added later, wear leveling belongs in a block driver, not in sexstore. |
| N2 | Multi-key atomic transactions | sexstore processes one message at a time. No concurrent multi-key operations exist. |
| N3 | Journal/redo log with rollback | The dual-page design provides crash atomicity without a journal. Simpler is safer. |
| N4 | Encryption at rest | No crypto material in scope. If encryption is added later, it belongs in a separate layer. |
| N5 | High-frequency write optimization | sexstore writes are infrequent (scene settings on preset change, config toggle). Write endurance is not a constraint. |
| N6 | Network-accessible storage | SexOS has no network stack. Durable is local only. |
| N7 | LIST/ENUM operations | Explicitly forbidden by E2–E9 gate. No iteration over stored keys. |

---

## 2. Backend Ownership Model

### Principle: sexstore is the sole durable backend authority

```
┌──────────────────────────────────────────────────┐
│                  sexstore (domain 8)              │
│                                                    │
│  ┌─────────────┐     ┌──────────────────────────┐ │
│  │  RAM K/V     │◄───►│  Durable Backend Module  │ │
│  │  (16 slots)  │     │  (dual-page or other)    │ │
│  └─────────────┘     └──────────┬───────────────┘ │
│                                  │                  │
│                                  ▼                  │
│                         ┌────────────────┐         │
│                         │  Raw Page I/O  │         │
│                         │ (kernel ABI)   │         │
│                         └────────────────┘         │
└────────────────────────────────────────────────────┘
```

### Ownership rules

1. **Only sexstore touches durable storage.** No other domain has a capability to the durable region. Silk-shell (domain 3) continues to access sexstore only via PDX protocol (PUT/GET/DEL messages).

2. **Durable is internal to sexstore.** No new protocol operations (no `OP_KV_DURABLE_READ`, no `OP_KV_DURABLE_SYNC`). The durable backend is invisible to clients.

3. **RAM is authoritative during runtime.** All PUT/GET/DEL operations read and write RAM first. The durable backend is updated synchronously after RAM commit.

4. **Boot loads durable → RAM.** On startup, sexstore reads all records from durable storage and populates the 16-slot RAM table. Any slot not present in durable storage defaults to Empty (state=0, gen=0).

5. **Write ordering:** RAM commit → CRC computation → durable page write → verify-after-write. If durable write fails, the operation is still reflected in RAM (current session works correctly), but a proof marker logs the failure.

### PDX protocol boundary (unchanged from E4–E10)

| Client | Protocol | Access | Durable aware? |
|--------|----------|--------|----------------|
| silk-shell (domain 3) | PUT/GET/DEL via PDX | SLOT_SEXSTORE cap | No — same API |
| All other domains | — | No capability | N/A |

---

## 3. Storage Unit Model

### DurableRecord format

Each durable record is a fixed-size 24-byte structure that stores the full state of one K/V slot:

```
Offset  Size  Field          Description
────── ────── ─────────────  ─────────────────────────────────────
 0      2     magic          Record magic: 0xD5 0xE5 (identifies valid record)
 2      1     version        Format version: 0x01
 3      1     flags          Flags: bit 0 = active (non-tombstone data present)
                                       bit 1 = tombstone (key was deleted)
                                       bit 2-7 = reserved (zero)
 4      2     slot_id        Slot index 0..15 (u16, for cross-check)
 6      2     crc16          CRC-16-IBM of bytes 0..5 + 8..23 (magic through val)
                                polynomial: x^16 + x^15 + x^2 + 1 (0x8005)
                                Initial value: 0x0000 (no XOR out)
                                NOTE: CRC field itself (bytes 6-7) set to 0 during computation
 8      1     state          Slot state: 0=Empty, 1=Active, 2=Tombstoned
 9      1     generation     Write generation (0=never written, 1..255, wraps 255→1)
10      2     pad            Reserved, zero
12      4     key            Stored key (u32)
16      8     val            Stored value (u64)
────── ────── ─────────────  ─────────────────────────────────────
Total: 24 bytes per record
```

### Record invariants

- `magic` must equal 0xD5E5 for a valid record. Any other value = uninitialized/corrupt slot.
- `version` must equal 0x01. Future versions must increment. Unknown versions are treated as corrupt.
- `slot_id` must be 0..15 and must match the slot position in the page layout.
- `crc16` covers the entire record structure (with CRC bytes zeroed during computation).
- `state` must be 0, 1, or 2. Any other value is corrupt.
- `generation` matches RAM slot generation for the same key.
- `key` and `val` are opaque — no validation at the durable layer (validation happens at the protocol layer before RAM commit).

### Page layout

Each page is a fixed-size region of memory (or emulated block). The Dual-Page Atomic Swap design uses two pages of equal size.

```
┌────────────────────────────────────────────┐
│  Page Header (16 bytes)                     │
│  ┌──────────┬──────┬──────┬──────────────┐ │
│  │ page_id  │ seq  │ crc  │ reserved(8)  │ │
│  │ u32=0xA5 │ u32  │ u32  │              │ │
│  └──────────┴──────┴──────┴──────────────┘ │
├────────────────────────────────────────────┤
│  Record 0  (24 bytes) — slot 0             │
│  Record 1  (24 bytes) — slot 1             │
│  ...                                        │
│  Record 15 (24 bytes) — slot 15            │
├────────────────────────────────────────────┤
│  Padding (zero-filled to page boundary)     │
└────────────────────────────────────────────┘
```

**Page size calculation:**

| Component | Size |
|-----------|------|
| Page header | 16 bytes |
| 16 records × 24 bytes | 384 bytes |
| Total payload | 400 bytes |
| Padding to next power-of-2 boundary | 112 bytes |
| **Total page size** | **512 bytes** |

Two pages = 1024 bytes total durable storage footprint. This fits comfortably within sexstore's existing static allocation constraints (no heap, no dynamic allocation). If 512-byte pages are too large, a 256-byte page (header + 10 records) is an alternative — but 512 bytes aligns with common block sizes.

**Page header fields:**

- `page_id: u32` — Fixed magic `0x0000A5A5` for active page identification.
- `seq: u32` — Monotonic sequence number. The page with the higher sequence number is the authoritative active page. Sequence numbers wrap at u32::MAX back to 1 (0 is reserved for uninitialized).
- `crc32: u32` — CRC-32 of the entire page (header + records + padding), with `crc32` field zeroed during computation. CRC-32C (Castagnoli) polynomial recommended for error detection.
- `reserved: [u8; 4]` — Zero. Reserved for future use (e.g., format version).

---

## 4. Candidate Comparison

### Candidate 1: Append-Only Bounded Log

| Property | Description |
|----------|-------------|
| **Mechanism** | A single linear region divided into fixed-size entries. New writes append to the log head. On wrap, the log is compacted: live entries are rewritten, dead entries (tombstoned/overwritten) are dropped. |
| **Storage** | Single region, e.g., 4096 bytes (240 entry slots at 17 bytes each). |
| **Crash safety** | Good — append is atomic if write granularity matches. On crash, replay from last valid entry. |
| **Boot load** | Scan entire log, apply newest entry per key to RAM. O(entries) scan. |
| **Compaction** | Required on wrap. O(entries) compaction. Must not lose data during compaction (needs double-buffer or checkpoint). |
| **Code complexity** | ~250 lines. Compaction is the hardest part — must handle power loss during compaction. |
| **Write amplification** | Low for normal writes (append only). High during compaction (rewrite all live entries). |
| **Wear leveling** | Poor — log head wears faster. |

**Verdict:** Viable but complex. Compaction adds risk and code surface. Not recommended for V1.

### Candidate 2: Fixed Checkpoint Page + Journal

| Property | Description |
|----------|-------------|
| **Mechanism** | A single checkpoint page (snapshot of all 16 slots) plus a journal of recent writes. On boot, load checkpoint, replay journal. On journal threshold, atomically rewrite checkpoint + clear journal. |
| **Storage** | Checkpoint: 512 bytes. Journal: 256 bytes (16 entries at 16 bytes each). Total: 768 bytes. |
| **Crash safety** | Good — journal is append-only. On crash, replay journal against checkpoint. |
| **Boot load** | Load checkpoint, then replay journal entries in order. Deterministic. |
| **Compaction** | Checkpoint rewrite on journal full. Must be atomic (dual-page swap for checkpoint write). |
| **Code complexity** | ~350 lines. Two structures, two write paths, journal replay logic. Most complex option. |
| **Write amplification** | Low for normal writes (journal append). Moderate on checkpoint rewrite. |

**Verdict:** Most flexible but highest complexity. Over-engineered for sexstore's current needs (16 slots, single writer, low frequency). Not recommended for V1.

### Candidate 3: Simple Slot Mirror (1:1 RAM mirror)

| Property | Description |
|----------|-------------|
| **Mechanism** | A direct 1:1 mirror of the 16-slot RAM table in a single page. Each PUT/GET/DEL writes the affected slot to the mirror at the same offset. |
| **Storage** | Single page: 16 records × 24 bytes + header = ~400 bytes, padded to 512 bytes. |
| **Crash safety** | Poor — a crash during a slot write leaves the mirror with a partial/corrupt record for one slot. No way to distinguish pre-write from post-write state at that slot. |
| **Boot load** | Read page, validate each record independently. Corrupt records must be discarded (default) — losing one slot's data. |
| **Compaction** | None needed — always a full snapshot. |
| **Code complexity** | ~60 lines. Simplest possible design. |
| **Write amplification** | 1:1 — exactly one record write per operation. |

**Verdict:** Simplest but crash-unsafe. A power loss during a slot write leaves ambiguity. Not acceptable for durable storage.

### Candidate 4: Dual-Page Atomic Swap (RECOMMENDED)

| Property | Description |
|----------|-------------|
| **Mechanism** | Two pages (A and B). Each page holds a full snapshot of all 16 slots + header with sequence number. On write, the inactive page is written with the new state, then toggled active via sequence number. |
| **Storage** | 2 × 512 bytes = 1024 bytes total. |
| **Crash safety** | **Excellent** — at any point, at least one page is fully consistent. A crash during page A write leaves page B as the authoritative snapshot. |
| **Boot load** | Read both pages, compare sequence numbers. Higher sequence = authoritative. Validate CRC and individual records. |
| **Compaction** | None needed — each page is always a full snapshot. |
| **Code complexity** | ~100 lines. No compaction, no journal, no replay. |
| **Write amplification** | 1 full page write per operation (512 bytes). Higher than mirror but still bounded. |
| **Durability guarantee** | Exactly-once semantics per completed operation. Power loss during write = previous snapshot preserved. |

**Verdict:** Best balance of crash safety, simplicity, and bounded footprint. Recommended for V1.

### Comparison matrix

| Criterion | Appended Log | Checkpoint+Journal | Slot Mirror | Dual-Page Swap |
|-----------|:---:|:---:|:---:|:---:|
| Crash safety | ★★★★ | ★★★★ | ★★ | ★★★★★ |
| Boot load speed | ★★★ (O(n)) | ★★★★ | ★★★★★ | ★★★★ |
| Code complexity | ★★ (~250) | ★ (~350) | ★★★★★ (~60) | ★★★★ (~100) |
| Write amplif. | ★★★★ | ★★★ | ★★★★★ | ★★★ |
| Compaction req. | ❌ Yes | ⚠️ Checkpoint | ❌ No | ❌ No |
| Storage footprint | 4096 B | 768 B | 512 B | 1024 B |
| Key recovery | Per-key latest | Checkpoint+journal | Per-slot latest | Full snapshot |
| **V1 suitability** | ⚠️ Viable | ❌ Over-engineered | ❌ Crash-unsafe | ✅ **RECOMMENDED** |

---

## 5. Recommended Backend: Dual-Page Atomic Swap

### Write flow (atomic)

```
PUT(key=K, val=V, caller=shell)
  │
  ├─ 1. RAM: update slot S with (K, V, gen+1, state=Active)
  │
  ├─ 2. DURABLE: compute new full snapshot
  │     ├─ Read all 16 RAM slots
  │     ├─ Build 16 DurableRecords
  │     ├─ Build page header with seq = active_page.seq + 1
  │     └─ Compute CRC32 of entire page
  │
  ├─ 3. WRITE: write snapshot to inactive page (the page with lower seq)
  │     ├─ memcpy or page_write(inactive_page_addr, snapshot, 512)
  │     └─ Verify-after-write: read back and compare CRC
  │
  ├─ 4. COMMIT: toggle active page
  │     └─ (implicit: inactive page now has higher seq → becomes active)
  │
  └─ 5. Proof marker: [sexstore.durable.write] slot=S key=K seq=N
```

### Boot flow (recovery)

```
sexstore init()
  │
  ├─ 1. Read page A header
  │     ├─ If header CRC valid AND page_id == 0xA5A5: seq_a = header.seq
  │     └─ Else: seq_a = 0 (invalid)
  │
  ├─ 2. Read page B header (same check)
  │     └─ seq_b = header.seq or 0
  │
  ├─ 3. Select authoritative page: higher seq wins
  │     ├─ If seq_a == 0 && seq_b == 0: no durable data (first boot)
  │     ├─ If seq_a > seq_b: page A is authoritative
  │     ├─ If seq_b > seq_a: page B is authoritative
  │     └─ If seq_a == seq_b && both > 0: both identical (normal after clean shutdown)
  │          → either page, no ambiguity
  │
  ├─ 4. Validate authoritative page CRC
  │     ├─ If CRC valid: perform per-record validation
  │     │     ├─ For each record: check magic, version, crc16, slot_id, state
  │     │     └─ Load valid records into RAM table
  │     └─ If CRC invalid: log [sexstore.durable.all_corrupt] and use defaults
  │
  ├─ 5. Initialize sexstore with loaded/default RAM table
  │
  └─ 6. Proof marker: [sexstore.durable.load] records=N valid=M corrupt=C
```

### Power-loss scenarios

| Scenario | State at power loss | Recovery outcome |
|----------|-------------------|------------------|
| Crash before inactive page write | Page A (seq=N) authoritative, Page B (seq=N-1) unchanged | RAM lost (volatile), but durable snapshot at seq=N loaded on boot — consistent |
| Crash during inactive page write (partial) | Page A (seq=N) valid, Page B has corrupt header/CRC | CRC check fails on Page B. Page A (seq=N) is authoritative. Last completed operation preserved. |
| Crash after write complete but before verify-read | Page A (seq=N) valid, Page B (seq=N) written but unverified | Boot selects page A (seq=N) or B (seq=N) — tie. Both identical if write completed. If verify-after-write failed, page B may be corrupt — CRCs catch it. |
| Crash during boot before RAM init | Durable state unchanged from last completed write | Reboot — load from durable again. Idempotent. |

### Write failure handling

If step 3 (write to inactive page) or verify-after-write fails:

1. **Operation still succeeds from client perspective** — RAM was already updated.
2. **Durable state is stale** — previous snapshot remains authoritative.
3. **Proof marker logged:** `[sexstore.durable.write.fail] slot=S key=K reason=write_fail`.
4. **Next durable write retries** — the inactive page from the failed write becomes the target again (it may contain partial data, so it's fully overwritten).

This means the durable store may lag behind RAM by at most one write in a failure scenario. The protocol guarantee is **at least once** durability for writes that complete their RAM commit. Clients can re-query to confirm.

---

## 6. Atomicity and Recovery Model

### Invariants

| # | Invariant | Enforcement |
|---|-----------|-------------|
| A1 | At least one page is always fully consistent | Dual-page design — pages are written independently. The active page (higher seq) is never overwritten until the inactive page write is verified. |
| A2 | Sequence numbers are monotonic and never zero | `seq` starts at 1 on first write. One page always has seq=0 (uninitialized) before first write. Wraps from u32::MAX to 1. |
| A3 | CRC-32C verification before page acceptance | Any page with invalid CRC is discarded. Boot and runtime verify-after-write both check CRC. |
| A4 | Per-record CRC-16 for slot-level integrity | Even if page CRC passes, each record's CRC-16 is verified independently. Catches intra-page corruption. |
| A5 | Empty/tombstoned slots are explicitly stored | A tombstoned slot in RAM is written as state=2 to the durable page. An empty slot is written as state=0. The durable store always reflects the full RAM state — not just active keys. |
| A6 | No partial page state observable | The write to the inactive page is a single operation (memcpy or page_write). If the write is interrupted, the page CRC fails on next boot. |

### Boot recovery pseudo-code

```
fn durable_load_all() -> Result<[Option<DurableRecord>; 16], DurableError> {
    let page_a = read_page(PAGE_A_ADDR);
    let page_b = read_page(PAGE_B_ADDR);

    let seq_a = if validate_page(&page_a) { page_a.header.seq } else { 0 };
    let seq_b = if validate_page(&page_b) { page_b.header.seq } else { 0 };

    let authoritative = if seq_a >= seq_b { page_a } else { page_b };

    if authoritative.header.seq == 0 {
        return Ok([None; 16]); // first boot — no durable data
    }

    let mut records = [None; 16];
    for i in 0..16 {
        let rec = authoritative.record(i);
        if rec.magic == 0xD5E5 && rec.version == 0x01 && rec.crc16_valid() {
            records[rec.slot_id as usize] = Some(rec);
        }
        // else: slot remains None → RAM default (Empty, gen=0)
    }

    Ok(records)
}
```

### Key recovery guarantee

After a crash + reboot, every key that was successfully written (PUT completed RAM commit + durable write started) is present in the loaded RAM table. Keys written in an operation that crashed before RAM commit are lost (consistent with volatile RAM semantics). Keys written in an operation that completed RAM commit but crashed during durable write are present in RAM after the operation but may not survive reboot — this is the **at least once** boundary:

```
Operation lifecycle:
  1. RAM commit     → survive runtime crash? ✅ YES (RAM is live until power loss)
  2. Durable start  → survive power loss?  ⚠️ Maybe (write may be partial)
  3. Durable commit → survive power loss?  ✅ YES (page written + verified)
```

For power-loss survival, the operation is durable after step 3. Step 2 is the window of vulnerability. The window is ~512 bytes of sequential write time — on real hardware, this is microseconds.

---

## 7. Privacy and Redaction Model

### Direct inheritance from E8

The E8 redaction policy applies identically to durable storage proof markers. No new redaction classes are introduced. The existing four-class hierarchy is sufficient:

| Class | Includes | Durable marker fields |
|-------|----------|----------------------|
| PublicProof | Marker name, status code, basic outcome | `durable.load`, `durable.write`, `durable.write.fail`, `durable.all_corrupt`, `durable.version` marker names and status |
| StructuralMeta | + caller PD, operation type, key (hashed/classified), slot state, generation | slot_id, key (as classified in E8 — not raw), state, generation, sequence number |
| SensitiveMeta | + object IDs, restore tokens, boot counts | Boot sequence number, page ID (⚠️ see below) |
| SecretContent | + stored values, paths, titles, crypto | **NEVER LOGGED** — not in proof markers, not in durable records |

### Boot sequence number sensitivity

The page header `seq` field is a **StructuralMeta** field when logged in proof markers:
- `[sexstore.durable.load] seq=127 records=12 valid=12 corrupt=0` — ✅ allowed (StructuralMeta)
- The sequence number itself is not user content — it's an internal counter.

### What durable markers may NOT log

| Forbidden field | Example | Why |
|----------------|---------|-----|
| Stored u64 value | `val=0xAC01010000000000` | SecretContent — directly contains user data (scene settings, future arbitrary values) |
| Raw key | `key=0x01` | StructuralMeta at marker level, but key classification must match E8. Key 0x01 is known (scene settings) — logging the raw key in markers is allowed per E7. Future arbitrary keys may need hashing. |
| User text | `title="My Document"` | SecretContent — never logged |
| Full page dump | Hex dump of durable page | SecretContent — contains stored values |

### Durable record contents are NEVER logged

The DurableRecord struct contains `val: u64` — this is a stored value and is SecretContent. Proof markers must reference records by slot_id + key + state only, never by value.

---

## 8. Proof Markers

### New durable-specific marker types

| Marker | Format | Budget | Class | Condition |
|--------|--------|--------|-------|-----------|
| `[sexstore.durable.write]` | `slot=S key=K seq=N state=S` | 16 per boot | StructuralMeta | Successful durable page write after PUT/GET/DEL |
| `[sexstore.durable.write.fail]` | `slot=S key=K reason=write_fail/timeout` | 8 per boot | StructuralMeta | Durable write failed (RAM still updated) |
| `[sexstore.durable.load]` | `seq=N records=R valid=V corrupt=C` | 1 per boot | PublicProof | Boot-time durable load complete |
| `[sexstore.durable.all_corrupt]` | `reason=crc_mismatch/version/header` | 1 per boot | PublicProof | Both pages corrupt — all defaults used |
| `[sexstore.durable.version]` | `ver=V` | 1 per boot | PublicProof | Durable format version mismatch (auto-repair attempted) |

### Marker budget impact

| Phase | Existing markers | New E11 markers | Total |
|-------|-----------------|-----------------|-------|
| E0 | 10 | — | 10 |
| E4 | 2 | — | 12 |
| E6 | 6 | — | 18 |
| E7 | 6 | — | 24 |
| E11 | — | 5 | **29** |

29 marker types total, 442 maximum per-boot (416 existing + 26 new: 16 write + 8 write.fail + 1 load + 1 all_corrupt + 1 version = 27, but write.fail is a subset of write budget).

### Marker field classification

All durable marker fields are classified per E8:

| Marker | Fields | E8 Class |
|--------|--------|----------|
| `durable.write` | slot, key, seq, state | StructuralMeta |
| `durable.write.fail` | slot, key, reason | StructuralMeta |
| `durable.load` | seq, records, valid, corrupt | PublicProof |
| `durable.all_corrupt` | reason | PublicProof |
| `durable.version` | ver | PublicProof |

No durable marker ever contains a stored value (SecretContent).

---

## 9. Migration Plan (E12 → E13)

### Phase E12: RAM-to-Durable Migration Spec (docs only)

**Timing:** After E11 approval.

**Scope:** Design the migration strategy for existing sexstore deployments that have been running with RAM-only storage (all current deployments). Key questions:

1. **First boot with durable enabled:** How does sexstore detect that durable storage is uninitialized (seq=0 on both pages)? Load RAM defaults (current behavior).

2. **Hot migration (runtime):** Can the durable backend be enabled at runtime without a reboot? Design a `durable_init()` call that writes the current RAM state to page A (seq=1) — this would be called once during sexstore init, always (if both pages are uninitialized).

3. **Cold migration (pre-existing RAM data):** Not applicable — RAM is volatile. Cold boot always starts from durable or defaults.

4. **Rollback:** If durable storage is corrupted or version-mismatched, fall back to RAM defaults. The `durable.all_corrupt` marker logs the event.

5. **Backward compatibility:** Durable format version 0x01 vs future versions. On version mismatch, log `durable.version` with the detected version, then treat as corrupt (defaults).

**No code changed in E12.** Spec only.

### Phase E13: Durable Backend Implementation (code)

**Timing:** After E12 approval.

**Scope:** Implement Dual-Page Atomic Swap in `servers/sexstore/src/main.rs`:

1. **Constants:** `PAGE_SIZE = 512`, `PAGE_A_BASE`, `PAGE_B_BASE`, `RECORD_SIZE = 24`, `HEADER_SIZE = 16`, `PAGE_ID_MAGIC = 0x0000A5A5`, `RECORD_MAGIC = 0xD5E5`, `FORMAT_VERSION = 0x01`.

2. **Structs:** `DurablePageHeader` (packed, repr(C)), `DurableRecord` (packed, repr(C)), `DurablePage` (512-byte aligned array).

3. **Functions:**
   - `durable_write_all(slots: &[KvSlot; 16]) -> Result<(), DurableError>` — Compute full snapshot, write to inactive page, toggle active.
   - `durable_load_all() -> Result<[Option<KvSlot>; 16], DurableError>` — Read both pages, select authoritative, validate, return records.
   - `durable_init()` — Called once at boot. If both pages uninitialized, write current RAM (defaults) as page A with seq=1.

4. **Integration points:**
   - `kv_put()`: After RAM commit + proof marker, call `durable_write_all()`.
   - `kv_del()`: Same — after RAM tombstone + marker, call `durable_write_all()`.
   - `init()`: After RAM table init, call `durable_load_all()` and populate RAM slots.

5. **Page I/O abstraction:** The actual page read/write mechanism depends on the kernel interface. Two options:
   - **Option A (no kernel change):** Use a statically allocated 1024-byte buffer in sexstore's BSS. This is RAM-backed but gives the code structure for future true durable I/O. The buffer is initialized from a fixed physical address (pre-allocated by bootloader or kernel).
   - **Option B (kernel ABI):** Add a new syscall or capability for persistent page I/O. This requires kernel changes and is NOT recommended for V1.

   **Recommendation:** Option A for V1 — use a battery-backed RAM region or emulated persistent region at a known physical address. The Dual-Page Atomic Swap logic is identical regardless of the backing store. The abstraction boundary is two functions:
   ```rust
   fn durable_page_read(page_id: u32, buf: &mut [u8; 512]) -> Result<(), ()>;
   fn durable_page_write(page_id: u32, buf: &[u8; 512]) -> Result<(), ()>;
   ```
   These can be implemented as memcpy from a fixed address for V1, or as a kernel ABI call in a future version.

### Implementation target

| Metric | Target |
|--------|--------|
| Added lines in sexstore/main.rs | ~120 lines |
| Added structs | 2 (`DurablePageHeader`, `DurableRecord`) |
| New functions | 3 (`durable_write_all`, `durable_load_all`, `durable_init`) |
| Page I/O abstraction | 2 functions (`page_read`, `page_write`) |
| New proof markers | 5 types |
| Storage footprint | 1024 bytes (2 × 512-byte pages) |
| No kernel changes | ✅ |
| No sex-pdx changes | ✅ |
| No protocol changes | ✅ |

---

## 10. Remaining Risks

| # | Risk | Severity | Mitigation | Status |
|---|------|----------|------------|--------|
| R1 | **Page I/O abstraction not hardware-backed** | MEDIUM | V1 uses RAM-based emulation (battery-backed region). True persistent storage requires hardware support. Documented as known limitation. | ⚠️ Accepted |
| R2 | **CRC-32C collision probability** | LOW | CRC-32C has Hamming distance 4 for 512-byte blocks. Probability of undetected error: ~2^-32 per page. Acceptable for non-critical config storage. | ✅ Acceptable |
| R3 | **Sequence number wrap ambiguity** | LOW | seq wraps from u32::MAX to 1. At 1 write/sec, this takes ~136 years to wrap. Even at 1000 writes/sec, ~49 days. After wrap, if one page has seq=1 and other has seq=MAX, MAX wins. Only ambiguous if both pages have the same seq (which shouldn't happen after clean shutdown — but if both have seq=1 after wrap, either page is authoritative). | ⚠️ Edge case — document |
| R4 | **Both pages corrupt simultaneously** | LOW | Dual ECC failure, cosmic ray, or firmware bug. Probability near-zero. Mitigation: fall back to RAM defaults. All scene settings reset — same as current behavior. | ✅ Acceptable |
| R5 | **Verify-after-write false positive** | LOW | If the read-back verification reads from cache instead of media, a write may appear successful when it wasn't. Mitigation: use cache-inhibited reads if available. For V1 RAM-backed, this is not an issue. | ⚠️ Document for hardware port |
| R6 | **Durable write slows PUT latency** | MEDIUM | Each PUT now does RAM commit + full page write (512 bytes). At realistic frequencies (human-scale config changes), this is invisible. At >100 writes/second, may need optimization. | ✅ Acceptable for V1 |
| R7 | **Reply buffer depth still 1** | MEDIUM | Deferred from E10. The durable write is synchronous (within the same message handler), so the depth-1 buffer is not exercised. If durable write becomes async in future, this must be fixed first. | 📄 Deferred remains |
| R8 | **No atomic multi-slot write** | LOW | Each PUT/GET/DEL touches exactly one slot. The full page write captures all 16 slots atomically. If two slots are updated in quick succession, each gets its own full page write — no race because sexstore is single-threaded. | ✅ Acceptable |

---

## 11. STOP FIRST Conditions

These conditions must be checked before any E11-related code is written (E13 implementation phase). If any condition is triggered, stop and require a new design document.

| # | Condition | Check | Stop? |
|---|-----------|-------|-------|
| S1 | Requires kernel ABI or syscall change | No new syscalls. Page I/O is internal to sexstore (fixed-address memcpy for V1). | ❌ Not triggered |
| S2 | Requires sex-pdx change | No protocol changes. Durable is internal. | ❌ Not triggered |
| S3 | Requires heap, alloc, or std dependency | All structs are statically sized, repr(C), packed. No alloc. | ❌ Not triggered |
| S4 | Adds LIST/ENUM or iteration protocol op | No new protocol operations. | ❌ Not triggered |
| S5 | Logs stored values, content, paths, or titles in proof markers | All durable markers logged as StructuralMeta or PublicProof per E8. No stored values. | ❌ Not triggered |
| S6 | Assumes POSIX filesystem or block device paths | Page I/O is fixed-address or kernel-provided region. No paths. | ❌ Not triggered |
| S7 | Makes durable storage authoritative over RAM during runtime | RAM remains authoritative. Durable is written after RAM commit. | ❌ Not triggered |
| S8 | Expands capability topology (new domains or caps) | Only sexstore touches durable region. No new caps. | ❌ Not triggered |
| S9 | Adds async operations that exercise the depth-1 reply buffer | Durable write is synchronous within the message handler. No async. | ❌ Not triggered |
| S10 | Implements encryption or crypto key management | No crypto. CRC-32C is error detection, not security. | ❌ Not triggered |
| S11 | Increases sexstore static allocation beyond available BSS | 1024 bytes for dual pages + struct overhead < 2 KB total. Sexstore has ample BSS. | ❌ Not triggered |
| S12 | Removes or bypasses the capability gate on any dispatch path | Cap gate (store_cap_allowed) remains on PUT/GET/DEL. Durable is internal. | ❌ Not triggered |

**All STOP FIRST conditions pass. E11 design is clear to proceed to E12 (migration spec) and E13 (implementation).**

---

## Appendix A: Header/Record Structure Definitions (Reference)

```rust
// Page header: 16 bytes
#[repr(C, packed)]
struct DurablePageHeader {
    page_id: u32,       // 0x0000A5A5 — identifies valid page
    seq: u32,           // monotonic sequence number (0 = uninitialized)
    crc32: u32,         // CRC-32C of entire page (header zeroed during compute)
    reserved: [u8; 4],  // zero
}

// Per-slot record: 24 bytes
#[repr(C, packed)]
struct DurableRecord {
    magic: u16,         // 0xD5E5 — record magic
    version: u8,        // 0x01 — format version
    flags: u8,          // bit 0 = active, bit 1 = tombstone
    slot_id: u16,       // 0..15 — slot index (cross-check)
    crc16: u16,         // CRC-16-IBM of record (crc16 field zeroed during compute)
    state: u8,          // 0=Empty, 1=Active, 2=Tombstoned
    generation: u8,     // write count (0=never, 1..255, wraps 255→1)
    pad: [u8; 2],       // zero
    key: u32,           // stored key
    val: u64,           // stored value
}

// Full page: 512 bytes
#[repr(C, align(512))]
struct DurablePage {
    header: DurablePageHeader,   // 16 bytes
    records: [DurableRecord; 16], // 384 bytes (24 × 16)
    padding: [u8; 112],          // pad to 512 bytes
}
```

Note: The `#[repr(C, packed)]` attribute ensures the struct layout matches the durable format exactly. Alignment (`align(512)`) ensures page alignment for potential future DMA or block I/O.

---

## Appendix B: CRC Selection Rationale

**Page-level CRC: CRC-32C (Castagnoli)**

- Polynomial: `0x1EDC6F41` (iSCSI, SCTP, ext4)
- Why: Better error detection than CRC-32 for Hamming distance and burst errors. Hardware-accelerated on modern x86 (SSE 4.2 `crc32` instruction).
- Implementation: For no_std, a 256-entry lookup table (1024 bytes) or bit-by-bit computation (~200 cycles/byte). For V1, bit-by-bit is acceptable for 512 bytes at boot time.

**Record-level CRC: CRC-16-IBM**

- Polynomial: `0x8005` (modbus, USB)
- Why: Compact (16-bit), sufficient for 24-byte records. Detects all 1-bit and 2-bit errors, all odd-bit errors, all burst errors ≤ 16 bits.
- Implementation: 256-entry lookup table (512 bytes) or bit-by-bit. 24 bytes × 16 records = 384 bytes total record CRC computation.

**Trade-off accepted:** Two CRC algorithms is slightly more code than one. The page CRC catches full-page corruption (power loss during write), while the record CRC catches per-slot memory corruption (bit flips in RAM-backed store). In V1 (RAM-backed dual pages), record CRC is redundant with page CRC but adds defense-in-depth at minimal code cost (~20 lines for table-less CRC-16).

---

## Appendix C: Write Amplification Worked Example

**Scenario:** User changes scene setting preset (1 PUT operation to key 0x01).

```
Before:
  Page A (seq=127, active): full snapshot of all 16 slots
  Page B (seq=126, inactive): previous snapshot

PUT(key=0x01, val=new_scene_blob):
  1. RAM: slot 0 updated (key=1, val=new_blob, state=Active, gen=42)
  2. Durable: compute new page with slot 0 updated
     → Write 512 bytes to Page B
     → Verify Page B CRC
  3. Toggle: Page B now has seq=128 → active

Total durable bytes written: 512
Total durable bytes read (verify): 512
Total I/O: 1024 bytes per PUT
```

**At realistic frequency:** 1 PUT per scene settings change (maybe 5-10 per session at most). Total durable I/O per session: ~5-10 KB. Well within any practical endurance limit.

---

## Appendix D: Key Architectural Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Full-page snapshot vs per-slot write | Full-page snapshot | Simpler crash model. No partial-write ambiguity. Only 512 bytes per write. |
| Two pages vs journal | Two pages | No journal replay logic. No compaction. At most one page write per operation. |
| CRC-32C vs SHA or HMAC | CRC-32C | Error detection is sufficient (no adversary model). CRC-32C is fast, compact, hardware-accelerated on x86. |
| Synchronous vs async durable write | Synchronous | sexstore is single-threaded. Sync write avoids reply-buffer depth issue. Async would require E11+ queue redesign. |
| RAM authoritative during runtime | RAM first | Durable failure does not affect current session. Degradation is graceful (stale durable state on next boot). |
| Fixed page layout vs variable | Fixed | 16 records always written. No compaction, no fragmentation, no allocation. Deterministic footprint. |

---

## Appendix E: References

| Document | Relevance |
|----------|-----------|
| `E10_MEDIUM_RISK_CLEANUP_V1.md` | Fixed caller constant, generation reclaim. Both relevant to durable record generation handling. |
| `E9_STORAGE_DURABLE_BACKEND_GATE_V1.md` | 10 entry criteria, 12 STOP FIRST conditions — all satisfied by this design. |
| `E8_STORAGE_REDACTION_POLICY_V1.md` | Redaction classes inherited directly. All durable markers classified. |
| `E6_STORAGE_TOMBSTONE_DELETE_V1.md` | KvSlot struct, state model, generation semantics. DurableRecord mirrors KvSlot. |
| `E4_STORAGE_VALUE_VALIDATION_V1.md` | Value envelope and validation. Durable layer stores opaque val — validation is at protocol layer. |
| `PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` | §16: E11 described as "Durable backend design." This document fulfills that milestone. |

---

*End of E11_DURABLE_BACKEND_DESIGN_V1.md*
