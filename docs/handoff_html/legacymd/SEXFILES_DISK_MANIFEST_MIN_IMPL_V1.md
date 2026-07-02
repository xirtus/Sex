# SEXFILES_DISK_MANIFEST_MIN_IMPL_V1

## Goal
Implement and prove minimal fixed DiskFS manifest mapping:
`/disk/sexfiles-proof-v1 -> LBA 2038..2045`, while preserving the existing `LBA 2047` guarded persistence lane.

## 1) Manifest Layout Implemented
In `servers/sexfiles/src/backends/diskfs.rs`:
- Fixed constants:
  - `DISKFS_MANIFEST_LBA = 2046`
  - `DISKFS_WRITE_PROOF_LBA = 2047`
  - `DISKFS_PROOF_OBJECT_START_LBA = 2038`
  - `DISKFS_PROOF_OBJECT_SECTORS = 8`
  - `DISKFS_MANIFEST_MAGIC = 0x31564D4B53494453` (`SDISKMV1` LE)
  - `DISKFS_MANIFEST_VERSION = 1`
  - `DISKFS_MANIFEST_ENTRY_MAX = 15`
- Fixed entry schema (`DiskManifestEntryV1`):
  - `name_hash: u64`
  - `start_lba: u64`
  - `len_bytes: u32`
  - `flags: u16`
- Helpers:
  - `proof_manifest_name_hash()` (FNV-1a 64-bit)
  - `proof_manifest_build_single_entry_sector()` (512-byte sector)
  - `proof_manifest_parse_single_entry()` with fail-safe validation

Validation enforced:
- bad magic/version -> reject
- entry_count bounds -> reject
- zero/oversized len -> reject
- collision with LBA 2046/2047 -> reject
- path hash mismatch -> reject
- unexpected start LBA -> reject

## 2) Reserved LBA Usage
- `LBA 2047`: unchanged guarded write proof slot (existing)
- `LBA 2046`: manifest sector (new)
- `LBA 2038..2045`: fixed object range (new)

## 3) Write Guard Adjustment (Narrow)
In `apps/sexdrive/src/main.rs`:
- `write_guard_allows()` extended only for proof mode (`buf_cap == SLOT_BUF_LEND`) to allow:
  - manifest: `LBA 2046` (`offset 0xffc00`, size 512)
  - object sectors: `LBA 2038..2045` (`offset 0xfec00..0xffa00`, size 512)
  - existing proof: `LBA 2047` (`offset 0xffe00`, size 512)
- No generic write range enabled.
- Added `nvme_write_one_block()` for non-2047 guarded writes.
- Existing `nvme_write_readback_proof()` retained for 2047 write+readback lane.

## 4) Proof Markers Observed
From serial logs:
- `[sexfiles.disk.manifest.write.begin] lba=2046`
- `[sexfiles.disk.manifest.write.ok] entries=1 path=/disk/sexfiles-proof-v1`
- `[sexfiles.disk.manifest.read.ok] lba=2046`
- `[sexfiles.disk.manifest.parse.ok] hash=0xdb0809f591d496d6 start_lba=2038 len=4096 flags=0x3`
- `[sexfiles.disk.object.write.ok] start_lba=2038 sectors=8`
- `[sexfiles.disk.object.read.ok] start_lba=2038 sectors=8`
- `[sexfiles.disk.object.match] path=/disk/sexfiles-proof-v1 start_lba=2038 sectors=8`

## 5) Object Payload Match Result
PASS: deterministic 8-sector payload written and read back byte-for-byte for `LBA 2038..2045`.

## 6) Persistence Proof Status
PASS after manifest/object implementation:
- Boot A still performs guarded `LBA 2047` write/readback.
- Boot B still shows read-before-write match:
  - `[sexfiles.persistence.boot_b.read_before_write.match] magic=0x3156455449525753 lba=2047 tag=0xa5a5a5a5a5a5a5a5`

## 7) Negative Test Status
PASS retained:
- `[sexfiles.storage.negative.summary] honest=1 ...`
- denied cases still denied.

## 8) Files Changed
- `servers/sexfiles/src/backends/diskfs.rs`
- `servers/sexfiles/src/proof.rs`
- `apps/sexdrive/src/main.rs`
- `docs/handoff/SEXFILES_DISK_MANIFEST_MIN_IMPL_V1.md`

## 9) Final Grep Commands
```bash
# Manifest/object proof
grep -E 'sexfiles\.disk\.manifest|sexfiles\.disk\.object|#PF|#GP|panic' .gate_master/serial.log

# Persistence + negatives after manifest changes
grep -E 'sexfiles\.persistence\.boot_b\.read_before_write\.(begin|match|mismatch)|sexfiles\.storage\.negative\.summary|#PF|#GP|panic' \
  .gate_master/serial.boot_a.log .gate_master/serial.boot_b.log

# Guarded write range evidence
grep -E 'sexdrive\.write\.guard\.(allow|deny)|sexdrive\.nvme\.write\.(submit|cqe|ok|err)' \
  .gate_master/serial.boot_a.log .gate_master/serial.boot_b.log
```

## 10) Next Prompt
- `SEXFILES_DISK_FILE_OPS_V1`
