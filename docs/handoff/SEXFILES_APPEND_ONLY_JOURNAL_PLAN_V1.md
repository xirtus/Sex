# SEXFILES_APPEND_ONLY_JOURNAL_PLAN_V1

## Status
Design-only handoff for next implementation round.

- No kernel changes
- No sex-pdx ABI changes
- No POSIX semantics
- No journal code implemented in this step

## Preconditions
- `SEXFILES_ON_DISK_FORMAT_LOCK_V1` completed.
- `DISKFS_SUPERBLOCK_OBJECT_TABLE_V1` completed as bounded in-memory scaffold with blocker documented for real persistence.

---

## 1. Journal Region Layout (V1)

Journal region is a fixed contiguous range from superblock:
- `journal_start_block`
- `journal_block_count`

V1 parameters:
- Block size: `4096`
- Record alignment: `16` bytes (header + payload + crc + padding)
- Append pointer stored in checkpoint metadata (not ABI-visible)

### Wrap/Clear behavior (V1)
V1 chooses deterministic **checkpoint-then-reset** behavior:
1. If next tx cannot fit in remaining journal region, attempt checkpoint.
2. If checkpoint succeeds, reset append pointer to journal start and continue.
3. If checkpoint cannot free enough space or object table update fails, return deterministic `ERR_FULL` / `ERR_OVERFLOW` class.

No circular partial overwrite in V1.

---

## 2. Record Types

Record type enum (u16):
- `TX_BEGIN = 1`
- `OBJECT_CREATE = 2`
- `OBJECT_WRITE_META = 3`
- `EXTENT_UPDATE = 4` (V1 replacement for data-ref update)
- `CAP_UPDATE = 5`
- `TX_COMMIT = 6`
- `CHECKPOINT = 7`

No delete/tombstone journal record in V1 journal plan unless object delete enters scope later.

---

## 3. Record Format

### Header (fixed, little-endian)
- `record_type: u16`
- `record_flags: u16` (V1 must be zero)
- `tx_id: u64`
- `object_id: u64` (`0` allowed for global/checkpoint records)
- `payload_len: u32`
- `record_generation: u64`
- `header_checksum: u32`

### Trailer
- `payload_checksum: u32`

### Checksum strategy
V1 implementation should use the same deterministic checksum family as selected for metadata in current scaffold for consistency and low risk in no_std.

---

## 4. Transaction Rules

1. Every tx starts with exactly one `TX_BEGIN(tx_id)`.
2. All data-bearing records for a tx share the same `tx_id`.
3. `TX_COMMIT(tx_id)` finalizes tx.
4. Replay applies only txs with valid `TX_BEGIN` and valid `TX_COMMIT`.
5. Incomplete txs are ignored.
6. Duplicate tx behavior is deterministic:
   - If same `tx_id` appears committed multiple times, apply only the first valid committed sequence and ignore later duplicates.
7. `record_generation` must be monotonic within a tx and non-decreasing across journal order.

---

## 5. Replay Algorithm (V1)

## Input
- Superblock
- Object table snapshot/checkpoint
- Journal region bytes

## Steps
1. Scan journal sequentially from journal start to append boundary.
2. For each candidate record:
   - Validate alignment and bounded `payload_len`.
   - Validate header checksum.
   - Validate payload checksum.
3. Build tx map keyed by `tx_id`:
   - Track `has_begin`, ordered records, `has_commit`, max generation.
4. Select committed txs only (`has_begin && has_commit`).
5. Sort committed txs by `(max_generation, tx_id)` for deterministic replay order.
6. Apply each record to in-memory object table state:
   - `OBJECT_CREATE`: reserve/create new object entry.
   - `OBJECT_WRITE_META`: update object metadata fields + generation.
   - `EXTENT_UPDATE`: update extent pointer/count metadata only.
   - `CAP_UPDATE`: update owner/rights_generation/rights bits.
7. Reject generation rollback attempts (`new_generation < current_generation`).
8. After replay, emit/derive checkpoint state and new monotonic fs generation.

## Corruption rule
- Corrupt record is skipped (not fatal to mount) if bounds do not imply region desync.
- If structural desync is detected (record length impossible to continue safely), stop scan and replay only txs collected up to prior valid boundary.

---

## 6. Failure Matrix

| Failure case | Rule | Result |
|---|---|---|
| Crash before `TX_BEGIN` persisted | No tx exists | No replay |
| Crash after `TX_BEGIN` before `TX_COMMIT` | Incomplete tx | Ignore tx |
| Crash during `TX_COMMIT` write | Commit checksum/length invalid | Ignore tx |
| Header/payload checksum mismatch | Record invalid | Skip record / stop-at-boundary if desync risk |
| Journal full | Checkpoint + reset attempt | If not recoverable: deterministic full error |
| Object table full during replay | Deterministic full/overflow error class | Mount may continue read-only behavior in later phase (policy to define) |
| Generation rollback attempt | Reject update | Keep prior object generation |

---

## 7. Proof Plan for Next Implementation

Required proof outcomes:
1. Committed tx replays (`TX_BEGIN + updates + TX_COMMIT`).
2. Uncommitted tx ignored.
3. Corrupt record rejected and not applied.
4. Generation monotonic enforcement.
5. Checkpoint selection and generation advancement deterministic.

Suggested markers for next round:
- `[diskfs.journal.proof.begin_commit]`
- `[diskfs.journal.proof.uncommitted_ignored]`
- `[diskfs.journal.proof.corrupt_rejected]`
- `[diskfs.journal.proof.generation_monotonic]`
- `[diskfs.journal.proof.checkpoint_select]`

---

## 8. Boundaries / Non-Goals

1. No Btrfs-style COW tree.
2. No ext4-style generalized journal modes.
3. No POSIX path/fsck behavior.
4. No app raw disk exposure.
5. No snapshot implementation in V1 journal phase.

---

## 9. Next Implementation Prompt Outline

`MISSION: DISKFS_APPEND_ONLY_JOURNAL_SCaffold_V1`

Scope:
1. Add bounded journal structs/constants in `sexfiles` backend.
2. Implement append path for `TX_BEGIN`, updates, `TX_COMMIT` in in-memory scaffold first.
3. Implement replay over in-memory journal bytes.
4. Add proof gate `SEXOS_DISKFS_JOURNAL_PROOF=1`.
5. Add markers from section 7.
6. Keep sexdrive/block persistence out unless safe route is explicitly wired and audited.

Stop-first triggers:
- Requires kernel/ABI change
- Requires broad backend rewrite
- Introduces POSIX semantics

---

## 10. Immediate Persistence Blocker (Unchanged)
Real durability remains blocked by missing safe sexfiles->sexdrive block I/O contract wiring for DiskFS backend.
This plan is implementable against current in-memory scaffold without changing ABI/kernel.

