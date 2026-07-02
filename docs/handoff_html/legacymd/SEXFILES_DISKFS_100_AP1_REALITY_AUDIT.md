# SEXFILES_DISKFS_100_AP1_REALITY_AUDIT

**Date:** 2026-05-22
**Phase:** AP1 — Audit
**Track:** SexFiles DiskFS bridge 100
**Predecessor:** sexdrive-storage-100-current-tier-v1 (closed, tagged)

---

## 1. BASELINE STATUS

| Item | Value |
|------|-------|
| HEAD commit | `5b4674e0 docs: close SexDrive storage current tier` |
| SexDrive closeout tag | `sexdrive-storage-100-current-tier-v1` — PRESENT |
| Working tree | Clean (only untracked .bak files) |
| SexDrive AP7 closeout | Committed + tagged ✓ |

---

## 2. FILES INSPECTED

| File | Relevance |
|------|-----------|
| `servers/sexfiles/src/main.rs` | Server entry, module declarations |
| `servers/sexfiles/src/vfs.rs` | VFS dispatch: RamFS + DiskFS bridge handlers |
| `servers/sexfiles/src/backends/diskfs.rs` | DiskFS backend: scaffold + real bridge methods |
| `servers/sexfiles/src/backends/mod.rs` | FsBackend trait definition |
| `servers/sexfiles/src/messages.rs` | OP_RAMFS_* and OP_DISKFS_* opcode constants |
| `servers/sexfiles/src/pdx.rs` | Re-exports of sex-pdx symbols |
| `servers/sexfiles/src/trampoline.rs` | Message loop: pdx_listen_raw → VFS → pdx_reply |
| `apps/sexdrive/src/main.rs` | SexDrive SLOT_BLOCK dispatch, NVMe read/write |
| `crates/sex-pdx/src/lib.rs` | SLOT_BLOCK, BLOCK_READ/WRITE/SYNC, MemLend protocol |

---

## 3. CURRENT SOURCE REALITY — PHASE C ANSWERS

### Q1: Does SexFiles currently have a DiskFS backend compiled and reachable?

**YES.** `servers/sexfiles/src/backends/diskfs.rs` is 2688 lines, compiled via `mod backends` → `pub mod diskfs`. The DiskFS module is imported in `vfs.rs` and its bridge methods are called directly from the VFS dispatch.

### Q2: Does DiskFS actually call SexDrive, or is it stubbed/mock/RamFS-only?

**BOTH — there are two parallel paths in the same file:**

| Path | Type | Reality |
|------|------|---------|
| In-memory scaffold (lines 1–1893) | Superblock, object_table, journal, checkpoints, extent allocator | **MOCK ONLY** — documented BLOCKER at lines 228–240. Operates on `DISKFS_STATE` RwLock, never touches NVMe. Used for format-lock and journal-replay proofs. |
| Bridge methods (lines 2137–2648) | `diskfs_write_object`, `diskfs_read_object`, `diskfs_lookup_path`, `diskfs_fsync`, `diskfs_ensure_manifest`, `diskfs_ensure_manifest_v2`, `diskfs_lookup_by_path_id` | **REAL** — call `diskfs_block_call()` which does `pdx_call(SLOT_BLOCK)` + `pdx_listen_raw(0)` to reach SexDrive. |
| FsBackend impl (lines 2650–2687) | `open`, `read`, `write`, `close`, `stat`, `list_at`, `len`, `create_with_owner` | **STUB** — all methods return `ERR_NOT_FOUND`. DiskFS is NOT accessible through the generic VFS handle-based path. |

**Critical nuance:** The out-of-date comment at line 228 says "BLOCKER: No real block I/O path is wired yet in sexfiles→sexdrive." This WAS true when written but is now STALE. The bridge methods added AFTER that comment DO wire real I/O. The scaffold (superblock/journal/etc.) remains in-memory only, but the block-level bridge is real.

### Q3: What slot/opcode/capability path does SexFiles use to reach SexDrive?

```
Linen PD → SLOT_STORAGE(1) → SexFiles trampoline → VFS dispatch
  → handle_diskfs_{write,read,flush,stat,manifest_hash,select}()
  → DiskFs::diskfs_{write,read}_object() / diskfs_fsync()
  → DiskFs::diskfs_block_call(opcode, offset, size, SLOT_BUF_LEND)
  → sex_pdx::pdx_call(SLOT_BLOCK(15), opcode, ...)
  → kernel IPC → SexDrive SLOT_BLOCK handler
  → nvme_write_one_block() / nvme_read_into_mapped_va()
  → real NVMe IO
```

**Capability path:**
- `sys_grant_mem_lend(SLOT_BLOCK, 4096, SLOT_BUF_LEND)` — SexFiles gets MemLend VA
- `sys_map_mem_lend(SLOT_BUF_LEND)` — SexDrive maps same MemLend VA
- `pdx_call(SLOT_BLOCK, BLOCK_READ/WRITE/SYNC, offset, size, SLOT_BUF_LEND)` — block command with buffer cap as arg2

### Q4: What buffer/data path is real?

**MemLend capability handoff — REAL.**

SexFiles:
1. Calls `sys_grant_mem_lend(SLOT_BLOCK, 4096, SLOT_BUF_LEND)` → gets `buf_va`
2. For write: copies data into `buf_va` via `core::ptr::write_volatile`
3. For read: clears `buf_va`, then after SexDrive fills it, copies out via `core::ptr::read_volatile`
4. Passes `SLOT_BUF_LEND` as `arg2` (buffer_cap) in `pdx_call`

SexDrive:
1. Validates `buf_cap == SLOT_BUF_LEND`
2. Calls `sys_map_mem_lend(SLOT_BUF_LEND)` → gets `fill_va` (same physical page)
3. Programs NVMe DMA to/from `fill_va`

**Bridge write data path:**
```
handle_diskfs_write() → packs 16 bytes from arg1+arg2 into inline_data
  → diskfs_write_object(path, offset, &inline_data, buf_va)
  → diskfs_lookup_path() reads manifest from NVMe
  → For each affected sector: read-modify-write via buf_va
  → diskfs_block_write(lba*512, 512, SLOT_BUF_LEND)
  → SexDrive nvme_write_one_block()
```

**Bridge read data path:**
```
handle_diskfs_read() → max 8 bytes
  → diskfs_read_object(path, offset, &mut rbuf[..rlen], buf_va)
  → diskfs_lookup_path() reads manifest from NVMe
  → diskfs_block_read(lba*512, 512, SLOT_BUF_LEND)
  → SexDrive nvme_read_into_mapped_va()
  → Copy sector bytes from buf_va → local buffer
  → Pack up to 8 bytes into reply u64 (LE)
```

**Limitations:** Bridge read returns max 8 bytes (fits in reply u64). Bridge write accepts max 16 bytes (two arg u64s). Multi-sector data path exists internally but the bridge API constrains it.

### Q5: Can SexFiles write one 512B block through SexDrive today?

**YES, at the code level.** `diskfs_write_object()` performs read-modify-write per sector, calling `diskfs_block_write()` → `pdx_call(SLOT_BLOCK, BLOCK_WRITE)` → SexDrive NVMe write. The `write_guard_allows()` in SexDrive permits writes to object LBAs (2022–2045 for multi-object V2, 2038–2045 for V1). This has NOT been proven end-to-end in a gate run.

### Q6: Can SexFiles read it back and compare bytes today?

**YES, at the code level.** `diskfs_read_object()` calls `diskfs_block_read()` → NVMe read. The bridge handler packs up to 8 bytes into the reply u64. Byte comparison would need to be done by the caller (Linen). Has NOT been proven end-to-end.

### Q7: Does current code support open/read/write/fstat/fsync semantics, or only block bridge?

**Bridge only.** The `DiskFs: FsBackend` implementation (lines 2650–2687) is a complete stub — every method returns `ERR_NOT_FOUND`. There is no handle-based filesystem API for DiskFS. RamFS has full handle semantics. DiskFS is accessed exclusively through the fixed-object bridge opcodes (0x38–0x3E).

### Q8: Is flush/fsync honest given SexDrive AP5b SKIP?

**YES.** The chain is:
1. `handle_diskfs_flush()` → `DiskFs::diskfs_fsync()` → `diskfs_block_sync()` → `pdx_call(SLOT_BLOCK, BLOCK_SYNC)`
2. SexDrive BLOCK_SYNC handler: returns `BLOCK_ERR_NO_DEVICE` with marker `[sexdrive.sync.recv] cmd=3 honest=flush_not_emulated_by_qemu_nvme`
3. SexFiles logs: `[sexfiles.bridge.diskfs.flush.err] status=4 honest=flush_not_emulated_by_qemu_nvme`
4. SexDrive has `nvme_flush()` function ready but commented out (QEMU doesn't post CQE for FLUSH)

The gate script's `sexfiles_diskfs_bridge` gate accepts `flush.(ok|err.*honest=)` as valid — correctly treating honest-error as PASS for flush.

### Q9: Are any existing gates claiming DiskFS/Linen persistence falsely?

**NO.** All three gate defaults are SKIP:
- `gate_linen_sexfiles100_audit="SKIP"`
- `gate_linen_diskfs_direct="SKIP"`
- `gate_sexfiles_diskfs_bridge="SKIP"`

The gate logic is well-designed: it detects fake success (write.ok + read.ok when backend returns no_ioq_ready), honest blockers, and violations (Linen calling SLOT_BLOCK directly). No false PASS is possible with these defaults.

### Q10: What is the smallest safe AP2?

**Run the existing bridge end-to-end.** All code is compiled and wired. AP2 needs:
1. A proof scenario that sends OP_DISKFS_SELECT → OP_DISKFS_WRITE → OP_DISKFS_READ → compare
2. Gate markers to classify the outcome
3. No new kernel/ABI/server code — just proof orchestration and gate classification

---

## 4. CURRENT TIER CLASSIFICATION — PHASE D

| Component | Status | Evidence Marker / Source Path |
|-----------|--------|-------------------------------|
| SexFiles server receives storage/VFS calls | **PROVEN** | `vfs.rs:369` dispatches RamFS + DiskFS bridge; `trampoline.rs:109` routes |
| DiskFS backend exists and compiles | **PROVEN** | `backends/diskfs.rs` — 2688 lines, `mod.rs:48` declares `pub mod diskfs` |
| DiskFS routes to SexDrive via SLOT_BLOCK | **PROVEN** | `diskfs.rs:261` `pdx_call(SLOT_BLOCK)` + `pdx_listen_raw(0)` |
| buffer/cap handoff path (MemLend) | **PROVEN** | `vfs.rs:46` `sys_grant_mem_lend(SLOT_BLOCK, 4096, SLOT_BUF_LEND)`; `sexdrive/main.rs:2630` `sys_map_mem_lend(SLOT_BUF_LEND)` |
| single-block write through SexFiles bridge | **PRESENT BUT UNPROVEN** | `diskfs_write_object()` at `diskfs.rs:2199`; `handle_diskfs_write()` at `vfs.rs:127`. Code compiles, no gate proof run. |
| single-block read through SexFiles bridge | **PRESENT BUT UNPROVEN** | `diskfs_read_object()` at `diskfs.rs:2314`; `handle_diskfs_read()` at `vfs.rs:216`. Code compiles, no gate proof run. |
| multi-block write/read through SexFiles | **PRESENT BUT UNPROVEN** | `diskfs_write_object()` handles read-modify-write across sectors (`diskfs.rs:2249-2302`). Bridge API limits to 16B write / 8B read per call. |
| reboot persistence through SexFiles | **STUB/MOCK** | `proof_reboot_persistence_roundtrip()` at `diskfs.rs:1296` operates on in-memory `DISKFS_STATE`, not real NVMe. Scaffold-only. |
| fsync/flush durability | **ABSENT (HONEST SKIP)** | `diskfs_fsync()` at `diskfs.rs:2404` calls BLOCK_SYNC; SexDrive returns `BLOCK_ERR_NO_DEVICE`. QEMU FLUSH CQE not posted. Commented-out `nvme_flush()` exists in SexDrive. |
| negative tests (bridge level) | **ABSENT** | Scaffold has journal/extent negative tests. Bridge has no negative classification. |
| V2 multi-object manifest | **PROVEN** | `diskfs_ensure_manifest_v2()` at `diskfs.rs:2528`; V2 entries for sexfiles-proof, linen-object, quil-object at LBAs 2038, 2030, 2022. Write guard allows all three ranges. |
| Linen client path to DiskFS bridge | **PRESENT BUT UNPROVEN** | `linen_diskfs_direct` gate exists at `gate.sh:3809` with PASS/FAIL/SKIP logic. Default SKIP. |
| RamFS as primary VFS backend | **PROVEN** | `vfs.rs:12` `RAMFS` static; `ramfs.rs` full implementation. All OP_RAMFS_* opcodes working. |

---

## 5. FALSE CLAIMS / GATE RISKS — PHASE E

**No false claims found.** All three DiskFS-related gates default to SKIP.

**Risk identified:** The `sexfiles_diskfs_bridge` gate (line 3855) will PASS if all success markers are present. This gate does NOT distinguish between:
- Real NVMe-backed success (bridge methods using `diskfs_block_call` → SexDrive)
- In-memory scaffold success (object_table/journal proofs)

However, the gate correctly checks for `no_ioq_ready` / fake-success patterns. The risk is theoretical since no runs have happened.

**Documentation debt:** The comment at `diskfs.rs:228-240` is stale. It claims "No real block I/O path is wired yet" but the bridge methods added afterward DO wire real I/O. This is misleading but not a gate risk.

---

## 6. STOP-FIRST BLOCKERS — PHASE F

| Blocker | Status |
|---------|--------|
| Kernel edits required? | **NO** — SLOT_BLOCK, SLOT_BUF_LEND, pdx_call, pdx_listen are all existing kernel syscalls. Bridge uses existing kernel ABI. |
| sex-pdx ABI edits required? | **NO** — BLOCK_READ/WRITE/SYNC opcodes, MemLend protocol are defined and stable. |
| Source reality contradicts expected path? | **NO** — the bridge path Linen→SexFiles→SexDrive→NVMe is wired and compiles. |
| False gate reports? | **NO** — all gates default SKIP. |
| Cross-PD raw pointers? | **NO** — MemLend uses kernel-granted capability handoff, not raw pointers across PD boundaries. |

**No blockers. AP2 can proceed.**

---

## 7. PROPOSED SEXFILES DISKFS 100 LADDER — PHASE G

Based on source reality, the following ladder is proposed. Adjustments from the template:

- **AP2 is NOT "write bridge code"** — the bridge already exists and compiles. AP2 is **prove the existing bridge works end-to-end**.
- **AP4 (reboot persistence)** must bridge through the block path. The in-memory scaffold reboot proof is insufficient.
- **AP6 (VFS open/read/write)** requires implementing the `FsBackend` trait for DiskFs, which currently returns ERR_NOT_FOUND on all methods. This is a larger scope than the template suggests.

### Proposed ladder:

| AP | Title | Scope |
|----|-------|-------|
| **AP1** | Reality audit | ✓ THIS DOCUMENT |
| **AP2** | Fixed-object bridge write/read/match through SexFiles→SexDrive | Run existing SELECT+WRITE+READ+compare for `/disk/sexfiles-proof-v1`. Prove 16B write/8B read matches. Gate classification: PASS/SKIP/FAIL. |
| **AP3** | Multi-object bridge: select+write/read for all 3 V2 paths | Prove `path_id=0,1,2` all resolve correctly. Write/read distinct patterns per-object to prove no cross-object contamination. |
| **AP4** | Fixed-object reboot persistence through bridge | Write known pattern, reboot QEMU (restart process), read back, compare. Prove data survives across QEMU restarts via real NVMe. NOT the in-memory scaffold reboot proof. |
| **AP5** | Negative bridge classification | Test: bad path_id, missing manifest, buffer grant failure, unaligned offset, past-end write. Prove errors propagate correctly. |
| **AP6** | Flush/fsync honest classification | Prove that OP_DISKFS_FLUSH→BLOCK_SYNC→BLOCK_ERR_NO_DEVICE chain is honest. Document QEMU limitation. Mark flush as honest SKIP (matching SexDrive AP5b). |
| **AP7** | Closeout / tag | Final gate run, classification table, tag: `sexfiles-diskfs-100-current-tier-v1` |

### Explicitly DEFERRED (NOT in this tier):

- Linen client integration (Linen proof tier separate)
- General VFS open/read/write semantics (requires FsBackend impl for DiskFs)
- Directory listing
- POSIX path semantics
- Power-loss durability (requires real NVMe controller with working FLUSH)
- DiskFS object table/journal/checkpoint on real NVMe (requires bridging scaffold to block path)
- Multi-PD concurrent access

---

## 8. RECOMMENDED NEXT AP2 TARGET — PHASE H

**AP2: Prove the existing SexFiles→SexDrive bridge works end-to-end.**

Target prompt:
```
MISSION: SEXFILES_DISKFS_100_AP2_BRIDGE_WRITE_READ_MATCH

Prove the existing DiskFS bridge pathway:
Linen → SLOT_STORAGE → SexFiles → DiskFS → SLOT_BLOCK → SexDrive → NVMe

Steps:
1. SELECT path_id=0 (/disk/sexfiles-proof-v1)
2. WRITE 16 bytes at offset 0 with known pattern
3. READ 8 bytes at offset 0
4. Compare — match must be exact
5. READ remaining 8 bytes at offset 8, compare
6. Emit proof markers for gate classification

No new code required for the bridge itself.
May need proof orchestration markers in Linen trampoline or SexFiles proof module.
Do NOT modify kernel, sex-pdx ABI, DiskFS, or SexDrive.

Gate: sexfiles_diskfs_bridge → PASS
Gate: linen_diskfs_direct → PASS (128B roundtrip variant)
```

---

## 9. EXACT GIT COMMANDS — PHASE I

```bash
git diff --stat
git add docs/handoff/SEXFILES_DISKFS_100_AP1_REALITY_AUDIT.md
git commit -m "docs: audit SexFiles DiskFS storage bridge reality"
```
