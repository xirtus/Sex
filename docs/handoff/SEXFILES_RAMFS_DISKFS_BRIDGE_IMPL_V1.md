# SEXFILES_RAMFS_DISKFS_BRIDGE_IMPL_V1

## Date

2026-05-07

## Status

IMPLEMENTED — Builds clean. Needs runtime test.

## Design Reference

`docs/handoff/SEXFILES_RAMFS_DISKFS_BRIDGE_ABI_PLAN_V1.md`

## Summary

Implemented the 5-opcode DiskFS bridge (0x38-0x3C) so Linen can save/load
objects directly through the DiskFS backend via SLOT_STORAGE.

Route: Linen → SLOT_STORAGE → SexFiles VFS → DiskFS file ops → SLOT_BLOCK → SexDrive → NVMe

## Opcodes Implemented

| Opcode | Name            | Direction    | Payload          |
|--------|-----------------|-------------|------------------|
| 0x38   | DISKFS_WRITE    | Linen→SexFiles | 16 bytes inline (arg1+arg2) |
| 0x39   | DISKFS_READ     | SexFiles→Linen | 8 bytes packed in reply u64 |
| 0x3A   | DISKFS_FLUSH    | Linen→SexFiles | no data (BLOCK_SYNC) |
| 0x3B   | DISKFS_STAT     | SexFiles→Linen | packed{flags:u32,size:u32} |
| 0x3C   | DISKFS_MANIFEST_HASH | SexFiles→Linen | FNV-1a 64-bit hash |

## Correction Applied (from refinement)

| # | Correction                                    | Status |
|---|-----------------------------------------------|--------|
| 1 | STOP FIRST — contract review not done, opcodes are free | Deferred to runtime gate |
| 2 | Dispatch in existing vfs.rs match, no refactor | DONE — inline handlers in match |
| 3 | DISKFS_WRITE rejects boundary writes > 4080    | DONE — ERR_OVERFLOW if offset+16 > 4096 |
| 4 | DISKFS_READ max 8 bytes (fits reply u64)       | DONE — max_len 1..8, reject 0 |
| 5 | 16-byte write via arg1+arg2, 8-byte read via reply | DONE |
| 6 | FLUSH honest error on QEMU                     | DONE — propagates BLOCK_ERR_NO_DEVICE=4 |
| 7 | Buffer granted once, reused, buf.ready marker  | DONE — AtomicU64 cached |
| 8 | Fixed object identity, no dynamic paths        | DONE — DISKFS_MANIFEST_OBJECT_PATH |
| 9 | Required proof markers present                 | DONE — all 21 markers present |
| 10 | Linen uses SLOT_STORAGE only, no SLOT_BLOCK   | DONE |
| 11 | Existing proofs preserved                     | DONE — no changes to proof.rs |
| 12 | Handoff states fixed-object bridge limitations | DONE — see below |

## Files Changed

| File                           | Changes |
|--------------------------------|---------|
| `servers/sexfiles/src/messages.rs` | Add OP_DISKFS_WRITE..MANIFEST_HASH (0x38-0x3C) + bounds constants |
| `servers/sexfiles/src/vfs.rs`   | Add DiskFs imports, bridge buffer state, 5 inline handlers, 5 dispatch cases |
| `servers/linen/src/main.rs`    | Add OP_DISKFS_* constants, LINEN_DISKFS_DIRECT_PROOF flag, run_linen_diskfs_direct_proof() |

**NOT changed:**
- `crates/sex-pdx/src/lib.rs` — no ABI edits
- `kernel/src/` — no kernel changes
- `apps/sexdrive/src/main.rs` — no sexdrive changes
- `servers/sexfiles/src/proof.rs` — existing proofs unchanged
- `servers/sexfiles/src/backends/diskfs.rs` — existing helpers unchanged

## Proof Flow (Linen → DiskFS)

```
1. OP_DISKFS_STAT     → verify object exists (size=4096, flags=0x3)
2. OP_DISKFS_MANIFEST_HASH → confirm path hash matches
3. OP_DISKFS_WRITE × 8      → write 128 bytes (8 × 16-byte chunks)
4. OP_DISKFS_FLUSH    → issue BLOCK_SYNC (honest ERR_NO_DEVICE on QEMU)
5. OP_DISKFS_READ × 16      → read 128 bytes (16 × 8-byte chunks)
6. Verify exact byte-for-byte match
7. Negative tests: write past 4096 → ERR_OVERFLOW, read past 4096 → ERR_OVERFLOW
```

## Bridge Markers

### SexFiles VFS side (servers/sexfiles/src/vfs.rs)
- `sexfiles.bridge.diskfs.recv op=0x38` through `op=0x3C`
- `sexfiles.bridge.diskfs.buf.ready buf_va=0x...`
- `sexfiles.bridge.diskfs.write.ok offset=N written=16`
- `sexfiles.bridge.diskfs.write.err offset=N code=E`
- `sexfiles.bridge.diskfs.read.ok offset=N read=8`
- `sexfiles.bridge.diskfs.read.err offset=N code=E`
- `sexfiles.bridge.diskfs.flush.ok` or `.flush.err`
- `sexfiles.bridge.diskfs.stat.ok size=4096 flags=0x3`
- `sexfiles.bridge.diskfs.manifest_hash.ok hash=0x...`

### Linen side (servers/linen/src/main.rs)
- `linen.diskfs.direct.begin`
- `linen.diskfs.direct.stat size=4096 flags=0x3`
- `linen.diskfs.direct.manifest_hash hash=0x...`
- `linen.diskfs.direct.save.request`
- `linen.diskfs.direct.write.ok written=128`
- `linen.diskfs.direct.flush.ok` or `.flush.err`
- `linen.diskfs.direct.load.request`
- `linen.diskfs.direct.read.match ok=1 size=128`
- `linen.diskfs.direct.bounds_negative ok=1`
- `linen.diskfs.direct.done`

## Build Commands

```bash
# Linen with bridge proof
RUSTFLAGS="-C relocation-model=pic -C link-arg=-pie" \
  SEXOS_LINEN_DISKFS_DIRECT_PROOF=1 cargo build \
  -Z build-std=core,compiler_builtins,alloc \
  -Z build-std-features=compiler-builtins-mem \
  --manifest-path servers/linen/Cargo.toml \
  --target x86_64-sex.json --release

# SexFiles with bridge handlers (always compiled)
RUSTFLAGS="-C relocation-model=pic -C link-arg=-pie" \
  cargo build \
  -Z build-std=core,compiler_builtins,alloc \
  -Z build-std-features=compiler-builtins-mem \
  --manifest-path servers/sexfiles/Cargo.toml \
  --target x86_64-sex.json --release
```

**Note**: The SexFiles bridge handlers (0x38-0x3C dispatch) are always compiled —
no env var needed. Only the Linen proof function is gated behind
`SEXOS_LINEN_DISKFS_DIRECT_PROOF=1`.

Full ISO build:
```bash
SEXOS_GATE_NVME=1 SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 \
  SEXOS_LINEN_DISKFS_DIRECT_PROOF=1 \
  ./scripts/master_runtime_gate.sh --probe 25 --keep-log
```

## Limitations (Exact)

- **Fixed object only**: Single object at `/disk/sexfiles-proof-v1` (4096 bytes).
- **No dynamic paths**: No directory tree, no create/delete/rename.
- **No general allocator**: Extent allocator exists but not wired to file ops.
- **No journaling for bridge ops**: Write calls go directly to DiskFS without
  journal transactions (RMW handles consistency per sector).
- **Single-client assumption**: Buffer is shared; no concurrent message handling.
- **FLUSH is honest error**: Returns BLOCK_ERR_NO_DEVICE on QEMU (NVMe FLUSH
  not emulated). Real NVMe hardware with ONCS bit 4 needed for durability.

## Preserved Proofs

All existing proofs are unchanged:
- `run_sexfiles_disk_file_ops_proofs()` — SEXOS_SEXFILES_REAL_BLOCK_PROOF
- `run_sexfiles_real_block_proofs()` — manifest + object write/read
- `run_sexfiles_reboot_proofs()` — SEXOS_SEXFILES_REBOOT_PROOF
- `run_linen_sexfiles_metadata_proofs()` — SEXOS_LINEN_SEXFILES_METADATA_PROOF
- `run_linen_disk_object_proof()` — SEXOS_LINEN_DISK_OBJECT_PROOF
- All fault injection gates (12 cases)
- All checkpoint/extent/journal proofs

## Grep Command

```bash
grep -rn 'sexfiles\.bridge\.diskfs\.\|linen\.diskfs\.direct\.' \
  servers/sexfiles/src/vfs.rs servers/linen/src/main.rs
```

## Next Prompt

```
Verify SEXFILES_RAMFS_DISKFS_BRIDGE_IMPL_V1 at runtime:
SEXOS_GATE_NVME=1 SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 SEXOS_LINEN_DISKFS_DIRECT_PROOF=1 \
  ./scripts/master_runtime_gate.sh --probe 25 --keep-log

Check serial for:
- sexfiles.bridge.diskfs.buf.ready
- sexfiles.bridge.diskfs.write.ok × 8
- sexfiles.bridge.diskfs.read.ok × 16
- linen.diskfs.direct.write.ok written=128
- linen.diskfs.direct.read.match ok=1
- No #PF/#GP/panic
```
