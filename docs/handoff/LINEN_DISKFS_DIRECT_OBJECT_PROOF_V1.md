# LINEN_DISKFS_DIRECT_OBJECT_PROOF_V1

## Date

2026-05-07

## Status

PROVEN — Direct Linen→DiskFS bridge roundtrip at runtime.

## Route

```
Linen (PD 7)
  │
  │ SLOT_STORAGE (slot 1, existing capability)
  │ OP_DISKFS_WRITE 0x38 (16 bytes inline via arg1+arg2)
  │ OP_DISKFS_READ  0x39 (8 bytes packed in reply u64)
  │ OP_DISKFS_FLUSH 0x3A
  │ OP_DISKFS_STAT  0x3B
  │ OP_DISKFS_MANIFEST_HASH 0x3C
  ▼
SexFiles (PD 11)
  │ VFS dispatch → handle_diskfs_*
  │ diskfs_ensure_manifest() → bootstrap if needed
  │ diskfs_bridge_get_buf_va() → sys_grant_mem_lend(SLOT_BLOCK, 4096, SLOT_BUF_LEND)
  │ diskfs_write_object() / diskfs_read_object()
  │ diskfs_block_write() / diskfs_block_read()
  │ SLOT_BLOCK (slot 15)
  ▼
SexDrive (PD 2)
  │ BLOCK_READ / BLOCK_WRITE handlers
  │ NVMe IO queue submission
  │ CQE polling
  ▼
NVMe device (QEMU nvme.img, LBAs 2038-2045 object, LBA 2046 manifest)
```

## Opcodes Used

| Opcode | Name            | Direction    | Payload per call   | Calls | Total payload |
|--------|-----------------|-------------|--------------------|-------|---------------|
| 0x3B   | STAT            | Linen←SexFiles | u64{flags,size}   | 1     | —             |
| 0x3C   | MANIFEST_HASH   | Linen←SexFiles | u64 FNV-1a hash    | 1     | —             |
| 0x38   | WRITE           | Linen→SexFiles | 16 bytes (arg1+2) | 8     | 128 bytes     |
| 0x3A   | FLUSH           | Linen→SexFiles | —                  | 1     | —             |
| 0x39   | READ            | Linen←SexFiles | 8 bytes (reply u64)| 16    | 128 bytes     |

## Manifest Bootstrap

On the first bridge WRITE or READ, `diskfs_ensure_manifest()` performs:
1. Read LBA 2046 via NVMe
2. Parse manifest via `proof_manifest_parse_single_entry`
3. If valid → cache "ready" flag, skip
4. If invalid → build known manifest sector, write to NVMe, read back to verify, cache "ready"
5. All subsequent bridge ops skip the check entirely

**Markers**: `manifest.ensure.begin`, `manifest.ensure.bootstrap` (first boot),
`manifest.ensure.valid` (subsequent boots), `manifest.ensure.ok`.

## Write Chunking

8 calls × 16 bytes each = 128 bytes total.

```
OP_DISKFS_WRITE offset=0   data_lo=payload[0..7]   data_hi=payload[8..15]  → 16
OP_DISKFS_WRITE offset=16  data_lo=payload[16..23]  data_hi=payload[24..31] → 16
OP_DISKFS_WRITE offset=32  ...  → 16
OP_DISKFS_WRITE offset=48  ...  → 16
OP_DISKFS_WRITE offset=64  ...  → 16
OP_DISKFS_WRITE offset=80  ...  → 16
OP_DISKFS_WRITE offset=96  ...  → 16
OP_DISKFS_WRITE offset=112 ...  → 16
```

## Read Chunking

16 calls × 8 bytes each = 128 bytes total.

```
OP_DISKFS_READ offset=0   max_len=8 → reply=payload[0..7]
OP_DISKFS_READ offset=8   max_len=8 → reply=payload[8..15]
... (all 16 offsets 0,8,16,...,120) ...
```

## Flush Result

`OP_DISKFS_FLUSH` returns 0 (OK). The NVMe write completes synchronously
(CQE received before write.ok). FLUSH is an additional durability barrier.
On QEMU, returns honest `BLOCK_ERR_NO_DEVICE` if not emulated.

## Bounds Negative

- Write at offset 4096 → `ERR_OVERFLOW` (rejected)
- Read at offset 4096 → `ERR_OVERFLOW` (rejected)

## Runtime Marker Chain (Verified)

```
linen.diskfs.direct.begin
linen.diskfs.direct.ready
linen.diskfs.direct.stat size=4096 flags=0x3
linen.diskfs.direct.save.request
sexfiles.bridge.diskfs.manifest.ensure.begin
sexfiles.bridge.diskfs.manifest.ensure.bootstrap
sexfiles.bridge.diskfs.manifest.ensure.ok
sexfiles.bridge.diskfs.write.ok ×8
linen.diskfs.direct.write.ok written=128
linen.diskfs.direct.flush.ok
linen.diskfs.direct.load.request
sexfiles.bridge.diskfs.read.ok ×16
linen.diskfs.direct.read.match ok=1 size=128
linen.diskfs.direct.bounds_negative ok=1 (write + read)
linen.diskfs.direct.done
No #PF/#GP/panic
```

## Safety Boundaries

| Boundary                          | Held? | How                                       |
|-----------------------------------|-------|-------------------------------------------|
| Linen uses SLOT_STORAGE only      | YES   | All bridge opcodes via pdx_call(SLOT_STORAGE) |
| Linen does not receive SLOT_BLOCK | YES   | No SLOT_BLOCK capability in Linen         |
| Linen does not receive MemLend    | YES   | No sys_grant_mem_lend calls in Linen      |
| Linen never calls SexDrive        | YES   | No SLOT_BLOCK, no BLOCK_* opcodes in Linen|
| SexFiles owns DiskFS policy       | YES   | diskfs_ensure_manifest, buffer mgmt in VFS|
| SexFiles owns internal MemLend    | YES   | sys_grant_mem_lend called in vfs.rs only  |
| No raw cross-PD pointers          | YES   | All data inline in u64 registers          |
| No shared-memory redesign         | YES   | Existing MemLend model unchanged          |

## Build Command

```bash
SEXOS_LINEN_DISKFS_DIRECT_PROOF=1 cargo build \
  -Z build-std=core,compiler_builtins,alloc \
  -Z build-std-features=compiler-builtins-mem \
  --manifest-path servers/linen/Cargo.toml \
  --target x86_64-sex.json --release
```

(sexfiles bridge handlers are always compiled, no env var needed.)

## Runtime Verification

```bash
SEXOS_GATE_NVME=1 SEXOS_LINEN_DISKFS_DIRECT_PROOF=1 \
  ./scripts/master_runtime_gate.sh --skip-build --probe 45 --keep-log

grep -E 'linen\.diskfs\.direct\.|sexfiles\.bridge\.diskfs\.' \
  .gate_master/serial.log
```

## Limitations (Exact)

- **Fixed object only**: Single object at `/disk/sexfiles-proof-v1` (4096 bytes).
- **No dynamic path IPC**: Object is implicit; no path strings in messages.
- **No general allocator**: Fixed LBAs (2038-2045 + manifest at 2046).
- **No directory tree/delete/rename**: Flat single entry manifest.
- **No journaling for bridge ops**: RMW per sector provides consistency.
- **Single-client**: Buffer is shared; no concurrent message handling.

## Next Roadmap

1. `FINAL_LINEN_DISKFS_BRIDGE_AUDIT_V1` — Final audit of the complete bridge stack.
2. `SEXFILES_DISK_MULTI_OBJECT_MANIFEST_PLAN_V1` — Plan multi-object manifest support.

## Canonical Claim

> Linen saves and loads a 128-byte object payload directly through the DiskFS
> backend at `/disk/sexfiles-proof-v1` using only its existing SLOT_STORAGE
> capability. The SexFiles server mediates all block I/O internally via
> SLOT_BLOCK and MemLend — Linen never sees or uses these capabilities.
> The manifest is bootstrapped on first use. Write/read roundtrip produces
> exact byte-for-byte match. Bounds are enforced. No isolation violations.
> No crashes. Fixed-object single-entry manifest V1.
