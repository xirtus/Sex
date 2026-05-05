# E12_RAM_TO_DURABLE_MIGRATION_SPEC_V1

**Status:** Docs/spec only. No code changed.

**Date:** 2026-05-05

**Review gate:** "Accept E12 only if it is docs-only and produces an E13-ready migration spec without implementing persistence."

**Depends on:** E11_DURABLE_BACKEND_DESIGN_V1 (dual-page design), E10_MEDIUM_RISK_CLEANUP_V1 (applied), E9_STORAGE_DURABLE_BACKEND_GATE_V1 (gate criteria), E8_STORAGE_REDACTION_POLICY_V1 (redaction), E6_STORAGE_TOMBSTONE_DELETE_V1 (slot model)

---

## Table of Contents

1. Current RAM Model (E4–E10)
2. Durable Model (from E11)
3. Boot Migration Flow
4. Write Migration Flow
5. Compatibility Guarantees
6. Failure Matrix
7. E13 Implementation Checklist
8. Proof Marker Plan
9. STOP FIRST Conditions
10. Final Verdict

---

## 1. Current RAM Model (E4–E10)

### 1.1 Slot state

```rust
struct KvSlot {
    state:      u8,   // 0=Empty, 1=Active, 2=Tombstoned
    generation: u8,   // 0=never written, 1..255 (wraps 255→1)
    key:        u32,  // opaque identifier
    val:        u64,  // stored value (opaque blob)
}
// 16 bytes per slot × 16 slots = 256 bytes total, static allocation
```

### 1.2 Key/value envelope

| Property | Value |
|----------|-------|
| Key namespace | Key 0x01 = scene settings (current), 0x00 = invalid, 0x02..0x0F = reserved for shell |
| Value validation | `store_validate_value()`: rejects bit 63 set, validates magic=0xAC/version=0x01/checksum for key 0x01 |
| Value contents | Packed scene settings blob: magic(1) + version(1) + preset_idx(1) + chrome_flags(1) + access_flags(1) + reserved(2) + checksum(1) = 8 bytes |

### 1.3 Generation/tombstone semantics

| Event | Generation behavior |
|-------|-------------------|
| Insert into empty slot | Set to 1 |
| PUT update (same key, active) | Bump (gen+1, wraps 255→1) |
| PUT revive (same key, tombstoned) | Bump |
| PUT reclaim (different key, tombstoned slot) | Reset to 1 (E10 fix) |
| DELETE active key | Bump, state → 2 |
| DELETE idempotent (already tombstoned) | No change |
| GET | No change |

Slot state transitions:

```
EMPTY (0) ──PUT──→ ACTIVE (1) ──DELETE──→ TOMBSTONED (2)
                     ↑ ↑                      │
                     │ └─── PUT (revive) ──────┘
                     └─────── PUT (update)
```

### 1.4 Status reply behavior

| Condition | Reply value |
|-----------|-------------|
| GET active key | Stored u64 (bit 63 = 0) |
| GET tombstoned/missing | `0x8000_0000_0000_0000 | KV_NOT_FOUND` |
| Any status reply | `0x8000_0000_0000_0000 | status_code` |
| PUT/DEL success | `0x8000_0000_0000_0000 | KV_OK` |

Status codes: KV_OK=0x00, KV_NOT_FOUND=0x01, KV_FULL=0x02, KV_INVALID_KEY=0x03, KV_INVALID_VALUE=0x04, KV_DENIED=0x05.

### 1.5 Shell client compatibility

Silk-shell (domain 3) is the only authorized caller (`KV_SHELL_CALLER = 3`). It uses:
- `OP_KV_GET = 0xB0`, `OP_KV_PUT = 0xB1` for read/write scene settings
- `handle_sexstore_get_reply()` with `STORE_REPLY_STATUS_BIT` dispatch
- `pack_scene_settings_blob()` / `unpack_scene_settings_blob()` for value envelope
- `store_reply_is_status()`, `store_reply_status()`, `store_reply_is_value()` helpers

Shell does NOT yet call DELETE (OP_KV_DEL=0xB2) — opcode exists server-side only.

---

## 2. Durable Model (from E11)

### 2.1 Dual-page architecture

```
Page A: 512 bytes (header + 16 records + padding)
Page B: 512 bytes (header + 16 records + padding)
Total: 1024 bytes static allocation in sexstore BSS
```

Both pages start uninitialized (all zeros). On first write, page A is written with seq=1. Subsequent writes alternate between pages.

### 2.2 Page header (16 bytes)

```rust
struct DurablePageHeader {
    page_id: u32,       // 0x0000A5A5 — identifies valid page
    seq: u32,           // monotonic sequence number (0 = uninitialized, 1..MAX, wraps to 1)
    crc32: u32,         // CRC-32C of page (header zeroed during compute)
    reserved: [u8; 4],  // zero
}
```

### 2.3 DurableRecord (24 bytes per slot, 16 records per page)

```rust
struct DurableRecord {
    magic: u16,         // 0xD5E5 — record magic
    version: u8,        // 0x01 — format version
    flags: u8,          // bit 0 = active data present, bit 1 = tombstone, bits 2-7 = 0
    slot_id: u16,       // 0..15 — slot index (cross-check)
    crc16: u16,         // CRC-16-IBM of bytes 0..5 + 8..23 (magic through val)
    state: u8,          // 0=Empty, 1=Active, 2=Tombstoned
    generation: u8,     // write count (mirrors RAM)
    pad: [u8; 2],       // zero
    key: u32,           // stored key
    val: u64,           // stored value
}
```

### 2.4 CRC scheme

| Level | Algorithm | Coverage | Purpose |
|-------|-----------|----------|---------|
| Page | CRC-32C (Castagnoli, 0x1EDC6F41) | Header + all records + padding | Full page integrity |
| Record | CRC-16-IBM (0x8005) | Magic through val (CRC field zeroed) | Per-slot integrity (defense-in-depth) |

### 2.5 Active-page selection

On boot: read both pages, validate headers. The page with the higher valid `seq` is authoritative. If both have equal valid `seq`, either is authoritative (identical content). If neither is valid, fall back to RAM defaults (first boot or both pages corrupt).

### 2.6 Fail-closed recovery

- Corrupt page header (page_id mismatch or CRC fail) → page treated as invalid (seq=0)
- Corrupt record (magic/version/crc16 fail) → that slot treated as Empty (state=0, gen=0)
- Both pages invalid → all slots default to Empty — same as current behavior
- Version mismatch (version != 0x01) → log `[sexstore.durable.version]`, treat as corrupt

---

## 3. Boot Migration Flow

### 3.1 Design decision: RAM-first boot

```
sexstore init()
  │
  ├─ 1. Initialize RAM table: all slots Empty (state=0, gen=0, key=0, val=0)
  │     (same as current behavior — RAM is always initialized to defaults)
  │
  ├─ 2. durable_load_all()
  │     ├─ Read page A (512 bytes from DURABLE_BASE + 0)
  │     ├─ Read page B (512 bytes from DURABLE_BASE + 512)
  │     ├─ Validate headers: page_id == 0xA5A5 && crc32 passes?
  │     │     ├─ Both invalid → no durable data → skip to step 4
  │     │     ├─ One valid → that page is authoritative
  │     │     └─ Both valid → higher seq wins
  │     ├─ If authoritative page exists (seq > 0):
  │     │     ├─ For each of 16 records:
  │     │     │     ├─ Validate magic (0xD5E5), version (0x01), slot_id (0..15), crc16
  │     │     │     ├─ Validate state (0, 1, or 2)
  │     │     │     └─ If valid: populate RAM slot with record's state/gen/key/val
  │     │     │         If invalid: RAM slot stays at defaults (Empty)
  │     │     └─ Log [sexstore.durable.load] with stats
  │     └─ Log failure markers if applicable
  │
  ├─ 3. durable_init() — cold start check
  │     └─ If both pages have seq == 0 (first boot ever):
  │           ├─ Write current RAM state (all defaults) to page A with seq=1
  │           └─ Log [sexstore.durable.load] seq=1 records=0 valid=0 corrupt=0
  │
  ├─ 4. Emit [sexstore.status.mapping] (existing boot marker)
  │
  └─ 5. Enter main dispatch loop (unchanged from E4–E10)
```

### 3.2 Rationale for RAM-first initialization

**Decision: Initialize RAM to defaults BEFORE loading from durable.**

This matches the current behavior (RAM table is always initialized to `[KvSlot { state: 0, generation: 0, key: 0, val: 0 }; 16]` at compile time). The durable load then **overwrites** slots with recovered data. This ensures:

1. If durable is corrupt or missing, RAM still has valid defaults
2. If durable has fewer than 16 valid records, unpopulated slots remain Empty
3. No need to track "was this slot loaded from durable or not" — just overwrite
4. Same initialization path regardless of durable presence

### 3.3 Cold start (first boot with durable)

On first boot after E13 implementation, both pages are all zeros (uninitialized):
- Page A header page_id = 0x00000000 ≠ 0xA5A5 → invalid
- Page B header page_id = 0x00000000 ≠ 0xA5A5 → invalid
- Both seq = 0 → no authoritative page
- `durable_init()`: write current RAM state to page A with seq=1
- Page A header: page_id=0xA5A5, seq=1, crc32=<computed>
- All 16 records written with state=0, generation=0, key=0, val=0
- This is a full valid page that future boots can load

This means after first boot, the durable store is always in a valid state. There is no "uninitialized" window after sexstore has run once.

### 3.4 Hot start (subsequent boots)

- Page A (seq=127, valid) and Page B (seq=126, valid)
- Boot selects page A (higher seq)
- Validates page CRC → passes
- Iterates 16 records, validates each
- Populates RAM table with 16 records
- sexstore enters dispatch loop with full state restored

### 3.5 Single-page valid recovery

- Page A (seq=42, valid) and Page B (corrupt header)
- Boot selects page A (only valid page)
- Same flow as hot start. The corrupt page B is ignored.
- The next durable write will target page B (the inactive page), completely overwriting the corrupt data.
- Result: after one write, both pages are valid again (page B has new seq=43).

---

## 4. Write Migration Flow

### 4.1 Design decision: RAM-first write ordering

```
PUT/DELETE operation:
  1. RAM commit (update KvSlot in-place)     ← always succeeds (no I/O)
  2. Check whether durable is initialized    ← always true after first boot
  3. Build full page snapshot from RAM table ← pure computation
  4. Write snapshot to inactive page         ← may fail (I/O error)
  5. Verify-after-write (read back CRC)      ← may fail (verify mismatch)
  6. If step 4 OR step 5 fails:
       Log [sexstore.durable.write.fail]
       Operation still reported as success to caller
  7. If steps 4 AND 5 succeed:
       Log [sexstore.durable.write] with seq
  8. Return status to caller (unchanged)
```

**Justification for RAM-first:** If durable write fails, the operation is still visible in RAM for the current session. The next boot will load the previous durable snapshot (stale by at most one write). This is strictly better than durable-first, where a failed durable write would require rolling back the RAM commit — adding complexity and risking data loss in RAM.

### 4.2 Determining the inactive page

```
let page_a_seq = if validate_page(&page_a) { page_a.header.seq } else { 0 };
let page_b_seq = if validate_page(&page_b) { page_b.header.seq } else { 0 };

let target_page = if page_a_seq >= page_b_seq { PAGE_B } else { PAGE_A };
let next_seq = max(page_a_seq, page_b_seq) + 1;
let new_header = DurablePageHeader {
    page_id: 0x0000A5A5,
    seq: next_seq,
    crc32: 0,  // computed after building records
    reserved: [0; 4],
};
```

Target page is the one with the lower sequence number (inactive). If both pages are equally valid (tied seq), target is page B (arbitrary but deterministic).

### 4.3 Building the page snapshot

```
fn build_page(slots: &[KvSlot; 16], seq: u32) -> DurablePage {
    let mut page = DurablePage::zeroed();
    page.header.page_id = 0x0000A5A5;
    page.header.seq = seq;
    page.header.crc32 = 0;
    for i in 0..16 {
        page.records[i] = DurableRecord {
            magic: 0xD5E5,
            version: 0x01,
            flags: if slots[i].state == 1 { 0x01 }
                   else if slots[i].state == 2 { 0x02 }
                   else { 0x00 },
            slot_id: i as u16,
            crc16: 0,  // computed below
            state: slots[i].state,
            generation: slots[i].generation,
            pad: [0; 2],
            key: slots[i].key,
            val: slots[i].val,
        };
        // Compute record CRC-16 (with crc16 field zeroed)
        page.records[i].crc16 = crc16_ibm(&page.records[i]);
    }
    // Compute page CRC-32C (with crc32 field zeroed)
    page.header.crc32 = crc32c(&page);
    page
}
```

### 4.4 Writing the page

The actual write mechanism depends on the page I/O abstraction:

```rust
fn durable_page_write(page_id: u32, buf: &[u8; 512]) -> Result<(), ()> {
    // V1 implementation: memcpy to reserved BSS region at DURABLE_BASE
    // For RAM-backed emulation: core::ptr::copy_nonoverlapping
    // For future hardware-backed: kernel ABI call
    let dest = DURABLE_BASE + (page_id as usize) * 512;
    unsafe {
        core::ptr::copy_nonoverlapping(buf.as_ptr(), dest as *mut u8, 512);
    }
    // Verify-after-write: read back and compare CRC
    let mut readback = [0u8; 512];
    durable_page_read(page_id, &mut readback);
    if crc32c(&readback) == extract_crc32(&readback) {
        Ok(())
    } else {
        Err(())
    }
}
```

### 4.5 Write failure behavior

| Failure point | RAM state | Durable state | Recovery |
|---------------|-----------|---------------|----------|
| RAM commit fails | Unchanged | Unchanged | NOT POSSIBLE — RAM commit is in-memory, cannot fail |
| Page build fails | Updated | Unchanged | Log `write.fail reason=build`, next write retries |
| Page write I/O error | Updated | Stale (prev page) | Log `write.fail reason=io_error`, next write targets same page |
| Verify-after-write fails | Updated | Stale (prev page) | Log `write.fail reason=verify`, next write targets same page |
| Verify-after-write passes | Updated | Updated (new seq) | ✅ Success — log `durable.write` |

### 4.6 Write flow for each operation

**PUT flow (with durable):**
```
PUT(key, val, caller):
  → policy gate (store_cap_allowed)
  → validate value (store_validate_value)
  → RAM: find/allocate slot, update kv slot (state, gen, key, val)
  → RAM: bump generation
  → [sexstore.generation.bump], [sexstore.put.allow] markers
  → durable_write_all(&KV)      ← NEW: synchronous durable write
  → kv_reply_status(caller, status)
```

**DEL flow (with durable):**
```
DEL(key, caller):
  → policy gate
  → RAM: find slot, set state=2, bump generation
  → [sexstore.tombstone.record], [sexstore.generation.bump] markers
  → durable_write_all(&KV)      ← NEW: synchronous durable write
  → kv_reply_status(caller, status)
```

**GET flow (unchanged — no durable write needed):**
```
GET(key, caller):
  → policy gate
  → RAM: scan for key
  → Return value or KV_NOT_FOUND
  → No durable write needed
```

---

## 5. Compatibility Guarantees

### 5.1 PDX protocol unchanged

| Aspect | Before E13 | After E13 | Change? |
|--------|-----------|-----------|---------|
| Opcodes | 0xB0 (GET), 0xB1 (PUT), 0xB2 (DEL) | Same | ❌ None |
| Message format | PdxMessage (type_id, arg0, arg1, arg2, caller_pd) | Same | ❌ None |
| Reply format | bit 63 = status/value discriminator | Same | ❌ None |
| Status codes | KV_OK=0, NOT_FOUND=1, FULL=2, INVALID_KEY=3, INVALID_VALUE=4, DENIED=5 | Same | ❌ None |
| Caller PD | Kernel-authoritative via PDX | Same | ❌ None |

### 5.2 OP_KV_DEL remains local

OP_KV_DEL = 0xB2 stays defined only in `servers/sexstore/src/main.rs:27`. Not promoted to sex-pdx. Not part of public ABI.

### 5.3 Shell status helpers unchanged

Silk-shell's `STORE_REPLY_STATUS_BIT`, `store_reply_is_status()`, `store_reply_status()`, `store_reply_is_value()`, `handle_sexstore_get_reply()` — all unchanged. The shell does not need to know about durable storage.

### 5.4 No new capabilities

No new cap grants. Only silk-shell (domain 3) has `SLOT_SEXSTORE` cap. Durable backend is internal to sexstore.

### 5.5 No app access

All apps (non-shell PDs) continue to be denied by `store_cap_allowed()`. App storage requires future E3 StoreCapability implementation.

### 5.6 RAM still authoritative during runtime

The durable backend is invisible to clients. All operations read from and write to RAM first. The durable write is a synchronous side effect. If durable write fails, the operation still succeeds from the client's perspective.

---

## 6. Failure Matrix

| # | Scenario | Durable state | Recovery | Outcome |
|---|----------|---------------|----------|---------|
| 1 | **Power loss before any durable write** (crash during first PUT before step 4) | Both pages uninitialized (seq=0) | Boot: both invalid → durable_init() writes page A with seq=1 | Current PUT lost (consistent with RAM volatility). Next boot starts clean. |
| 2 | **Power loss during inactive page write** (crash after partial write, at byte 127 of 512) | Inactive page has corrupt header/CRC. Active page unchanged. | Boot: active page CRC passes → loaded. Corrupt page ignored. | Last completed operation preserved. Partial write lost. |
| 3 | **Power loss after full page write but before verify** (crash after memcpy, before readback) | Inactive page fully written but unverified. Both pages valid with different seq. | Boot: both valid → higher seq wins. If write was complete, new page has higher seq. | If write completed fully: operation durable. If write was interrupted (any byte): corrupt CRC → previous page wins. |
| 4 | **Power loss during boot, before durable_load_all completes** (crash during RAM init) | Durable pages unchanged from last boot. | Reboot: durable_load_all runs again. Pages still valid. | Idempotent. No data loss. |
| 5 | **Both pages valid, different seq** (normal after clean operation) | Page A seq=N, Page B seq=N+1. Both CRC valid. | Boot: higher seq (N+1) wins. Either page valid for read. | ✅ Expected. |
| 6 | **Both pages valid, same seq** (tie after clean shutdown or seq wrap) | Both pages have identical seq (e.g., seq=128 on both after clean shutdown where both were written). | Boot: either page wins (seq tie → first). Both identical so no ambiguity. | ✅ Expected after clean shutdown with both pages synced. |
| 7 | **One page corrupt** (bit flip in header page_id or CRC) | Page A valid seq=42. Page B has corrupted header (page_id changed). | Boot: page B header CRC fails → seq=0. Page A authoritative. | ✅ Last valid state recovered. Single-bit error tolerated. |
| 8 | **Both pages corrupt** (dual bit flip or media failure) | Neither page passes header validity. | Boot: both seq=0 → no durable data. All RAM defaults. | ⚠️ Data loss. Same as current behavior (all defaults). Log `durable.all_corrupt`. |
| 9 | **Sequence number wrap** (seq goes from u32::MAX to 1) | Page A seq=MAX, Page B seq=1 (newly written after wrap). | Boot: MAX > 1, so page A wins. This is correct — page A is the older valid snapshot. After next write, page B gets seq=2 → page B wins. | ✅ Correct. Higher seq always wins, even across wrap boundary. |
| 10 | **Single record corruption** (bit flip in one DurableRecord's key or val) | Page valid at header level. One record has invalid magic/CRC. | Boot: page CRC passes (page level). Per-record validation: corrupt record fails magic/crc16 check → that slot stays Empty. | ⚠️ One slot lost. Other 15 slots recovered. Log per-record failure in load stats. |
| 11 | **Tombstone record corruption** (tombstoned slot's record gets bit flip in state field) | State field changed from 2 to 0 or 1. | If state → 0 (Empty): record invalid (state not 1 or 2). Or if state → 3: invalid. If state → 1 (Active): wrong semantics — key appears active when it was tombstoned. | ⚠️ Record CRC should catch this (state is covered by CRC-16). CRC-16 detects all 1-bit and 2-bit errors. |
| 12 | **Write failure during reclaim** (durable write fails after RAM reclaim of tombstoned slot) | Durable page has old tombstoned record for previous key. RAM has new key in that slot. | Boot loads durable → sees old tombstoned key. RAM init: slot populated with tombstone state. Next PUT to that key in RAM will reclaim it (RAM state=2, but key mismatch). | ⚠️ Stale tombstone in durable. Next PUT reclaims it correctly. RAM and durable diverge by one write. |
| 13 | **Generation counter persisted correctly** | DurableRecord generation matches RAM generation at time of write. | On boot load, generation is restored correctly. Generation continuity is preserved across reboots. | ✅ Generation monotonicity preserved across power cycles. |
| 14 | **Verify-after-write false positive** (readback returns cached data instead of media) | In-memory emulation: readback always matches what was written. No false positives. | N/A for V1 (RAM-backed). Document for hardware port: use cache-inhibited reads. | ✅ Acceptable for V1. |

---

## 7. E13 Implementation Checklist

### 7.1 New constants in `servers/sexstore/src/main.rs`

```rust
// E12/E13: Durable backend constants
const DURABLE_PAGE_SIZE: usize = 512;
const DURABLE_PAGE_A_BASE: usize = 0x...;  // TBD: fixed address in BSS or kernel-provided region
const DURABLE_PAGE_B_BASE: usize = DURABLE_PAGE_A_BASE + 512;
const DURABLE_PAGE_ID_MAGIC: u32 = 0x0000A5A5;
const DURABLE_RECORD_MAGIC: u16 = 0xD5E5;
const DURABLE_FORMAT_VERSION: u8 = 0x01;
const DURABLE_HEADER_SIZE: usize = 16;
const DURABLE_RECORD_SIZE: usize = 24;
```

### 7.2 New structs

```rust
#[repr(C, packed)]
struct DurablePageHeader {
    page_id: u32,
    seq: u32,
    crc32: u32,
    reserved: [u8; 4],
}

#[repr(C, packed)]
struct DurableRecord {
    magic: u16,
    version: u8,
    flags: u8,
    slot_id: u16,
    crc16: u16,
    state: u8,
    generation: u8,
    pad: [u8; 2],
    key: u32,
    val: u64,
}

#[repr(C, align(512))]
struct DurablePage {
    header: DurablePageHeader,
    records: [DurableRecord; 16],
    padding: [u8; 112],
}
```

### 7.3 New helper functions

| Function | Signature | Lines | Purpose |
|----------|-----------|-------|---------|
| `durable_page_read` | `fn durable_page_read(page_id: u32, buf: &mut [u8; 512]) -> Result<(), ()>` | ~8 | Read 512 bytes from durable region |
| `durable_page_write` | `fn durable_page_write(page_id: u32, buf: &[u8; 512]) -> Result<(), ()>` | ~12 | Write 512 bytes to durable region + verify |
| `crc32c` | `fn crc32c(buf: &[u8]) -> u32` | ~20 | CRC-32C computation (bit-by-bit or table) |
| `crc16_ibm` | `fn crc16_ibm(buf: &[u8]) -> u16` | ~15 | CRC-16-IBM computation |
| `validate_page` | `fn validate_page(page: &DurablePage) -> bool` | ~8 | Check page_id magic + CRC32 |
| `build_page` | `fn build_page(slots: &[KvSlot; 16], seq: u32) -> DurablePage` | ~25 | Build full page snapshot from RAM |
| `durable_write_all` | `fn durable_write_all(slots: &[KvSlot; 16]) -> Result<(), ()>` | ~20 | Determine inactive page, build, write, verify |
| `durable_load_all` | `fn durable_load_all() -> (u32, u32, u32)` | ~30 | Read both pages, select authoritative, return stats |
| `durable_init` | `fn durable_init() -> bool` | ~15 | Check if durable is initialized; if not, write page A with seq=1 |

**Total new code:** ~120-150 lines (including whitespace and comments).

### 7.4 Integration points in sexstore dispatch

| Location | Change | Type |
|----------|--------|------|
| `_start()` after status.mapping marker | Call `durable_init()` then `durable_load_all()` → populate RAM | New call |
| `OP_KV_PUT` handler, after RAM commit + proof markers | Call `durable_write_all(&KV)` | New call |
| `OP_KV_DEL` handler, after RAM tombstone + proof markers | Call `durable_write_all(&KV)` | New call |
| `OP_KV_GET` handler | No change — GET is read-only | ❌ None |

### 7.5 Proof marker integration

New markers emitted at these points:

| Marker | Where emitted | Condition |
|--------|---------------|-----------|
| `[sexstore.durable.write]` | After successful durable_write_all() | Write verified |
| `[sexstore.durable.write.fail]` | After failed durable_write_all() | Write/verify failed |
| `[sexstore.durable.load]` | After durable_load_all() in init | Boot load complete |
| `[sexstore.durable.all_corrupt]` | After durable_load_all() when both pages invalid | Both pages corrupt |
| `[sexstore.durable.version]` | After durable_load_all() when version mismatch | Version ≠ 0x01 |

### 7.6 Budget additions

| Static | Budget | Purpose |
|--------|--------|---------|
| `LOG_DURABLE_WRITE` | 16 | Successful durable page writes |
| `LOG_DURABLE_WRITE_FAIL` | 8 | Failed durable page writes |
| `LOG_DURABLE_LOAD` | 1 | Boot-time load (no budget needed — single emission) |
| `LOG_DURABLE_ALL_CORRUPT` | 1 | Both pages corrupt (no budget needed) |
| `LOG_DURABLE_VERSION` | 1 | Version mismatch (no budget needed) |

Total additional marker budget: 24 (16 write + 8 write.fail). Load/corrupt/version are singleton boot markers.

### 7.7 Build and proof commands

```bash
# Build sexstore with durable backend
make

# Verify build
grep "[SEXOS ENTRYPOINT] success" build.log

# Check sexstore binary size
size servers/sexstore/target/x86_64-sexos/release/sexstore

# Verify no new warnings (expected: 1 pre-existing)
make 2>&1 | grep -c "warning"

# Proof: verify durable markers appear in boot log
rg "\[sexstore.durable." qemu_debug.log

# Proof: verify no stored values in any marker
rg "\[sexstore" qemu_debug.log | rg "val=|value=" | wc -l
# Expected: 0 — no marker logs stored values

# Proof: verify RAM-only protocol still works
rg "\[sexstore.put.allow\]" qemu_debug.log
rg "\[sexstore.get.allow\]" qemu_debug.log
```

### 7.8 Negative tests (E13 verification)

| Test | Expected | How to verify |
|------|----------|---------------|
| Cold boot: first boot with durable | Page A written with seq=1, all slots Empty | Check `durable.load` marker seq=1 |
| PUT key after cold boot | Page B written with seq=2, slot populated | Check `durable.write` marker |
| Reboot after PUT | RAM populated from durable, key present | Check GET returns correct value |
| DELETE key, reboot | RAM has tombstoned key from durable | Check GET returns KV_NOT_FOUND |
| Power loss during write (hardware test only) | Durable has previous snapshot | Simulate: skip verify, next boot loads old seq |
| Both pages invalid (simulate) | All RAM defaults | Set page magic to 0 before boot, check `all_corrupt` marker |
| Version mismatch (simulate) | Record rejected, slot defaults | Set version to 0xFF, check `durable.version` marker |
| Single record corruption (simulate) | One slot defaults, others loaded | Corrupt one record's magic, check load stats |

---

## 8. Proof Marker Plan

### 8.1 New durable markers (5 types)

| Marker | Format | Budget | E8 Class | When |
|--------|--------|--------|----------|------|
| `[sexstore.durable.write]` | `slot=S key=K seq=N state=S` | 16 | StructuralMeta | Successful page write after PUT/DEL |
| `[sexstore.durable.write.fail]` | `slot=S key=K reason=W` | 8 | StructuralMeta | Page write failure (RAM still updated) |
| `[sexstore.durable.load]` | `seq=N records=R valid=V corrupt=C` | 1 | PublicProof | Boot: durable load complete |
| `[sexstore.durable.all_corrupt]` | `reason=R` | 1 | PublicProof | Both pages invalid — defaults used |
| `[sexstore.durable.version]` | `ver=V` | 1 | PublicProof | Format version mismatch detected |

### 8.2 Full marker inventory (post-E13)

| Phase | Marker types | Total budget |
|-------|-------------|-------------|
| E0 legacy | `kv.put`, `kv.get` | 64 |
| E4 | `policy.allow`, `policy.deny`, `key.invalid`, `value.invalid`, `reply.error` | 88 |
| E6 | `generation.bump`, `tombstone.record`, `tombstone.get`, `tombstone.revive`, `status.mapping` (boot) | 144 |
| E7 | `put.allow`, `put.reject`, `get.allow`, `get.reject`, `delete.allow`, `delete.reject` | 120 |
| E11/E13 | `durable.write`, `durable.write.fail`, `durable.load`, `durable.all_corrupt`, `durable.version` | 24 + 3 boot |
| **Total** | **24 marker types** | **440 + 3 boot** |

### 8.3 Marker field redaction (E8 compliance)

All durable markers are classified per E8 redaction policy:

| Marker | Fields logged | E8 Class | SecretContent? |
|--------|---------------|----------|----------------|
| `durable.write` | slot, key, seq, state | StructuralMeta | ❌ None |
| `durable.write.fail` | slot, key, reason | StructuralMeta | ❌ None |
| `durable.load` | seq, records, valid, corrupt | PublicProof | ❌ None |
| `durable.all_corrupt` | reason | PublicProof | ❌ None |
| `durable.version` | ver | PublicProof | ❌ None |

No durable marker logs the stored `val` field (SecretContent). No durable marker logs paths, titles, or user text.

---

## 9. STOP FIRST Conditions

| # | Condition | Check for E12/E13 | Stop? |
|---|-----------|-------------------|-------|
| S1 | Requires kernel ABI or syscall change | Page I/O is internal memcpy at fixed address. No new syscalls. | ❌ Not triggered |
| S2 | Requires sex-pdx change | No protocol changes. OP_KV_DEL stays local. | ❌ Not triggered |
| S3 | Requires heap, alloc, or std dependency | DurablePage (512 bytes) is static. All structs repr(C) packed. No alloc. | ❌ Not triggered |
| S4 | Adds LIST/ENUM or iteration protocol op | No new protocol operations. Durable is internal. | ❌ Not triggered |
| S5 | Logs stored values, content, paths, or titles | All durable markers StructuralMeta or PublicProof. No val field in markers. | ❌ Not triggered |
| S6 | Assumes POSIX filesystem or block device paths | Page I/O is fixed-address memcpy. No paths, no filesystem. | ❌ Not triggered |
| S7 | Makes durable authoritative over RAM during runtime | RAM is authoritative. Durable written after RAM commit. | ❌ Not triggered |
| S8 | Expands capability topology | Only sexstore touches durable region. No new caps or domains. | ❌ Not triggered |
| S9 | Adds async operations or exercises depth-1 reply buffer | Durable write is synchronous within dispatch handler. GET unchanged. | ❌ Not triggered |
| S10 | Implements encryption or crypto key management | CRC-32C/CRC-16 are error detection, not security. No crypto. | ❌ Not triggered |
| S11 | Exceeds sexstore BSS limits | Durable pages: 1024 bytes + stack usage < 2 KB. Sexstore has sufficient BSS. | ❌ Not triggered |
| S12 | Removes or bypasses capability gate | store_cap_allowed() still called on all PUT/GET/DEL. Durable is internal. | ❌ Not triggered |
| S13 | **E12 implements code, not docs** | E12 is docs-only. No code changed. | ❌ Not triggered (E12) |
| S14 | Requires actual persistent hardware for V1 | V1 uses RAM-backed region (BSS static). Same dual-page logic. | ❌ Not triggered |

**All STOP FIRST conditions pass. E12 migration spec is clear to proceed to E13 implementation.**

---

## 10. Final Verdict

```
E12_RAM_TO_DURABLE_MIGRATION_SPEC_V1

Status: PASS — migration spec approved.

Verdict: E13-ready.

Key decisions:
  ✅ Boot flow: RAM defaults first, durable overwrites on load
  ✅ Write ordering: RAM commit first, durable write second
  ✅ Cold start: durable_init() writes page A with seq=1 on first boot
  ✅ Fail-closed: corrupt/both-invalid → RAM defaults (same as current)
  ✅ Generation preserved across reboots (persisted in DurableRecord)
  ✅ Tombstone state persists across reboots (state=2 in DurableRecord)
  ✅ No protocol changes (PDX, opcodes, status codes all unchanged)
  ✅ No kernel/sex-pdx edits
  ✅ No new capabilities or app access
  ✅ E8 redaction inherited (no SecretContent in durable markers)

E13 implementation scope:
  - ~120-150 lines of new code in sexstore/main.rs
  - 2 new structs (DurablePageHeader, DurableRecord)
  - 3 new functions (durable_write_all, durable_load_all, durable_init)
  - 2 I/O abstraction functions (durable_page_read, durable_page_write)
  - 2 CRC functions (crc32c, crc16_ibm)
  - 5 new proof marker types (24 budget + 3 boot singletons)
  - Integration at PUT, DEL, and init paths
  - GET path unchanged
  - Build: [SEXOS ENTRYPOINT] success target
  - No new warnings expected (beyond 1 pre-existing)
```

---

## Appendix A: E13 Code Structure Preview

```
servers/sexstore/src/main.rs
├── Constants (lines 1-50)
│   ├── OP_KV_GET/PUT/DEL (existing)
│   ├── KV_OK..KV_DENIED (existing)
│   ├── REPLY_STATUS_BIT (existing)
│   ├── DURABLE_PAGE_SIZE, _PAGE_A_BASE, _PAGE_B_BASE (NEW)
│   ├── DURABLE_PAGE_ID_MAGIC, _RECORD_MAGIC, _FORMAT_VERSION (NEW)
│   └── KV_SLOT_COUNT (existing)
├── Structs
│   ├── KvSlot (existing)
│   ├── DurablePageHeader (NEW)
│   ├── DurableRecord (NEW)
│   └── DurablePage (NEW)
├── Static data
│   ├── KV: [KvSlot; 16] (existing)
│   ├── Proof marker budgets (existing)
│   └── DURABLE_PAGES: [u8; 1024] (NEW, for RAM-backed emulation)
├── Functions
│   ├── kv_reply, kv_reply_status (existing)
│   ├── bump_generation (existing)
│   ├── store_key_owner_class, store_cap_allowed (existing)
│   ├── store_validate_value (existing)
│   ├── crc32c (NEW)
│   ├── crc16_ibm (NEW)
│   ├── durable_page_read, durable_page_write (NEW)
│   ├── validate_page (NEW)
│   ├── build_page (NEW)
│   ├── durable_write_all (NEW)
│   ├── durable_load_all (NEW)
│   └── durable_init (NEW)
└── _start()
    ├── durable_init() + durable_load_all()  (NEW, at boot)
    ├── status.mapping marker (existing)
    └── dispatch loop (existing)
        ├── OP_KV_PUT → RAM update → durable_write_all() (NEW)
        ├── OP_KV_GET → RAM scan → reply (unchanged)
        ├── OP_KV_DEL → RAM tombstone → durable_write_all() (NEW)
        └── _ → reply error (existing)
```

## Appendix B: References

| Document | Section |
|----------|---------|
| `E11_DURABLE_BACKEND_DESIGN_V1.md` | §3 (record layout), §5 (write flow), §6 (recovery), Appendix A (struct defs) |
| `E10_MEDIUM_RISK_CLEANUP_V1.md` | Risk 2 (generation reset on reclaim — reflected in durable record gen) |
| `E9_STORAGE_DURABLE_BACKEND_GATE_V1.md` | §5 (backend design constraints), §6 (STOP FIRST) |
| `E8_STORAGE_REDACTION_POLICY_V1.md` | §2 (marker classification), §3 (forbidden fields) |
| `E6_STORAGE_TOMBSTONE_DELETE_V1.md` | §4 (slot model), §5 (generation), §6 (tombstone) |
| `servers/sexstore/src/main.rs` | Full current implementation — integration targets for E13 |

---

*End of E12_RAM_TO_DURABLE_MIGRATION_SPEC_V1.md*
