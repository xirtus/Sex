# SEXFILES_BRIDGE_MANIFEST_BOOTSTRAP_RUNTIME_V1

## Date
2026-05-07

## Status
PASS — Bridge write/read roundtrip verified at runtime without REAL_BLOCK_PROOF.

## Implementation

Added `DiskFs::diskfs_ensure_manifest(buf_va)` in `backends/diskfs.rs`:
1. Reads LBA 2046 (manifest sector) via NVMe
2. Parses the manifest via `proof_manifest_parse_single_entry`
3. If valid (correct magic, version, entry for /disk/sexfiles-proof-v1):
   emits `manifest.ensure.valid`, returns Ok
4. If invalid (garbage, uninitialized NVMe):
   emits `manifest.ensure.bootstrap`, builds and writes the known fixed
   manifest sector, reads back to verify, emits `manifest.ensure.ok`
5. Idempotent: second call sees valid manifest, skips write

Added manifest-ready cache (`DISKFS_MANIFEST_READY: AtomicU64`) in vfs.rs:
- Set to 1 after first successful `diskfs_ensure_manifest()`
- Subsequent WRITE/READ ops skip the manifest check entirely
- Avoids redundant NVMe manifest reads (significant perf win)

## Runtime Verification

```
SEXOS_GATE_NVME=1 SEXOS_LINEN_DISKFS_DIRECT_PROOF=1
./scripts/master_runtime_gate.sh --skip-build --probe 45 --keep-log
```

### Full Marker Chain

```
[linen.diskfs.direct.begin]
[linen.diskfs.direct.ready]
[sexfiles.bridge.diskfs.recv] op=0x3B stat              ← STAT
[linen.diskfs.direct.stat] size=4096 flags=0x3
[sexfiles.bridge.diskfs.recv] op=0x3C manifest_hash       ← HASH
[sexfiles.bridge.diskfs.recv] op=0x38 offset=0            ← WRITE #1
[sexfiles.bridge.diskfs.manifest.ensure.begin]            ← bootstrap starts
[sexfiles.bridge.diskfs.manifest.ensure.bootstrap]        ← NVMe was empty
[sexfiles.bridge.diskfs.manifest.ensure.ok]               ← manifest written
[sexfiles.bridge.diskfs.write.ok] offset=0 written=16     ← WRITE #1 OK
[sexfiles.bridge.diskfs.write.ok] offset=16 written=16    ← WRITE #2 OK (cached)
... (offsets 32,48,64,80,96,112 — all 16 bytes) ...
[linen.diskfs.direct.write.ok] written=128                ← 8×16 = 128
[sexfiles.bridge.diskfs.recv] op=0x3A flush               ← FLUSH
[linen.diskfs.direct.flush.ok]
[linen.diskfs.direct.load.request] offset=0 size=128
[sexfiles.bridge.diskfs.read.ok] offset=0 read=8          ← READ #1
[sexfiles.bridge.diskfs.read.ok] offset=8 read=8          ← READ #2
... (offsets 0,8,16,24,32,40,48,56,64,72,80,88,96,104,112,120 — all 8 bytes) ...
[linen.diskfs.direct.read.match] ok=1 size=128            ← MATCH
[linen.diskfs.direct.bounds_negative] ok=1 test=write_past_end
[linen.diskfs.direct.bounds_negative] ok=1 test=read_past_end
[linen.diskfs.direct.done]
```

### Counts
- 8/8 writes (all 16 bytes)
- 16/16 reads (all 8 bytes)
- 1 manifest bootstrap (first write only)
- 0 redundant manifest checks (cache hit on all subsequent ops)
- 2/2 bounds negative tests
- 0 #PF/#GP/panic

## Files Changed

| File | Change |
|------|--------|
| `servers/sexfiles/src/backends/diskfs.rs` | +110: `diskfs_ensure_manifest()` with read/parse/bootstrap/verify |
| `servers/sexfiles/src/vfs.rs` | +20: manifest-ready cache + ensure calls in write/read handlers |

## Bootstrap Policy

- **Idempotent**: valid manifest → no-op. Invalid → write once.
- **Single-entry only**: manifest contains exactly one entry for `/disk/sexfiles-proof-v1`.
- **Does NOT touch LBA 2047** (persistence proof slot preserved).
- **Does NOT require REAL_BLOCK_PROOF**: works on clean NVMe image.
- **Verify-after-write**: reads back and re-parses to confirm correctness.

## Next Prompt

`LINEN_DISKFS_DIRECT_OBJECT_PROOF_V1` — the bridge is now fully functional.
Or commit all bridge files together.
