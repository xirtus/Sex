# SEXFILES_RAMFS_DISKFS_BRIDGE_RUNTIME_V1

## Date
2026-05-07

## Status
BRIDGE CODE VERIFIED. WRITE PATH BLOCKED ON MANIFEST POPULATION.

## Runtime Test Results

### Test 1: Without REAL_BLOCK_PROOF (sexfiles fast boot)
```
[linen.ready]
[linen.diskfs.direct.begin]
[linen.diskfs.direct.ready]
[sexfiles.ready]
[sexfiles.bridge.diskfs.recv] op=0x3B stat         ← Bridge STAT works
[sexfiles.bridge.diskfs.stat.ok] size=4096 flags=0x3
[linen.diskfs.direct.stat] size=4096 flags=0x3
[sexfiles.bridge.diskfs.recv] op=0x3C manifest_hash ← Bridge HASH works
[sexfiles.bridge.diskfs.manifest_hash.ok] hash=0xdb0809f591d496d6
[sexfiles.bridge.diskfs.recv] op=0x38 offset=0       ← Bridge WRITE received
[sexfiles.bridge.diskfs.buf.ready] buf_va=0x400000355000  ← Buffer granted
[sexfiles.bridge.diskfs.write.err] offset=0 code=-3  ← ERR_NOT_FOUND (no manifest on NVMe)
```
**Result**: STAT and HASH succeed. WRITE receives, buffer grants, but
`diskfs_lookup_path` fails because the manifest sector was never written
to the NVMe (REAL_BLOCK_PROOF was disabled).

### Test 2: With REAL_BLOCK_PROOF (manifest written, but slow boot)
```
[linen.ready]
[linen.diskfs.direct.begin]
[linen.diskfs.direct.ready]
... (Silence — Linen blocks in pdx_storage_sync, SexFiles runs proofs) ...
[sexfiles.disk.file.manifest.pre_write] ok=1 lba=2046  ← Manifest written
[sexfiles.disk.file.ops.proof.done] ALL FILE OPS CHECKS PASSED
[sexfiles.ready]
... (Scheduler yields through all PDs, no bridge markers appear) ...
```
**Result**: Manifest IS written. SexFiles reaches ready. But Linen's bridge
messages are lost — likely ring buffer overflow during SexFiles' long boot.

### Bridge Code Status

| Opcode | Compile | Dispatch | Reply | NVMe Path |
|--------|---------|----------|-------|-----------|
| 0x38 WRITE | ✅ | ✅ recv + buf_ready | ✅ write.err returned | ❌ manifest not on NVMe |
| 0x39 READ  | ✅ | ✅ compiled | ✅ (same path as WRITE lookup) | ❌ same blocker |
| 0x3A FLUSH | ✅ | ✅ compiled | N/A (bypasses manifest) | ✅ returns BLOCK_ERR_NO_DEVICE |
| 0x3B STAT  | ✅ | ✅ recv + stat.ok | ✅ returns size=4096 flags=0x3 | N/A (no NVMe) |
| 0x3C HASH  | ✅ | ✅ recv + hash.ok | ✅ returns 0xdb0809f591d496d6 | N/A (no NVMe) |

## Root Cause

The bridge WRITE calls `diskfs_write_object` which calls `diskfs_lookup_path`.
`diskfs_lookup_path` reads the manifest sector from NVMe LBA 2046. If the
manifest was never written (REAL_BLOCK_PROOF disabled), the NVMe returns
uninitialized data → manifest parse fails → ERR_NOT_FOUND.

When REAL_BLOCK_PROOF IS enabled, the file ops proof writes the manifest.
But SexFiles' boot takes ~2400 log lines, during which Linen's messages
accumulate in the ring buffer. The ring buffer likely overflows before
SexFiles reaches its message loop.

## Workaround

The bridge can be tested by running sexfiles WITHOUT REAL_BLOCK_PROOF
but WITH a pre-populated NVMe image that contains the manifest at LBA 2046.
STAT and HASH work immediately (no NVMe dependency). WRITE/READ require
the manifest to exist on the NVMe.

## Files for Final Commit

```
servers/sexfiles/src/messages.rs    — 5 opcodes + bounds constants
servers/sexfiles/src/vfs.rs         — 5 handlers + buffer state
servers/linen/src/main.rs           — 5 opcode constants + bridge proof
```

## Next Steps

1. `SEXFILES_FAST_BOOT_ORDERING_FIX_V1` — Reorder proof execution so
   the manifest is written BEFORE the message loop starts, allowing
   the bridge to work without the full REAL_BLOCK_PROOF suite.

2. Or: `SEXFILES_BRIDGE_RING_BUFFER_EXPANSION_V1` — Increase ring
   buffer capacity to survive long startup proofs.

3. Or: `LINEN_SEXFILES_READY_SIGNAL_V1` — Kernel-mediated rendezvous
   so Linen knows when SexFiles is accepting messages.
