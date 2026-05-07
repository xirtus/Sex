# LINEN_DISKFS_SLOT_OBJECT_PROOF_V1

## Status: COMPLETE

16B app-boundary proof: Linen writes/reads through SLOT_STORAGE to
its DiskFS V2 object at path_id=1 (/disk/linen-object-v1, LBA 2030-2037).

## Architecture

```
Linen (PD 7) → SLOT_STORAGE → SexFiles (PD 11) → DiskFS → SexDrive → NVMe
                     ↑ AsyncEnqueue (Domain cap)     ↑ SyncCall (IPC cap)
```

Key finding: `SLOT_STORAGE` uses `AsyncEnqueue` (Domain cap edge), NOT
`SyncCall`. The `pdx_call` return value `r` is always the enqueue ack (0),
never the reply. All reply data must be obtained via `pdx_listen_raw(0)`.

## Implementation

### Gate: `cfg!(linen_diskfs_slot_proof)`
Set via `sexos_build_spec.toml`:
```toml
[[stage]]
id = "build_linen"
rustflags = "--cfg linen_diskfs_slot_proof"
```

### Proof Function: `run_linen_diskfs_slot_proof()`
1. Cooperative yield readiness wait (64× `sched_yield()`)
2. SELECT path_id=1 (OP_DISKFS_SELECT=0x3E) — blocking reply via `storage_sync_reply()`
3. STAT (OP_DISKFS_STAT=0x3B) — blocking reply
4. WRITE 1×16B payload "LINEN-SLOT-V1!\0\x01" — blocking reply
5. READ 2×8B chunks — blocking reply for each
6. Byte-for-byte match verification

`storage_sync_reply()`: blocks on `pdx_listen_raw(0)`, forwards HID events,
returns reply value when `type_id == 0x1`.

## Proof Markers
```
[linen.diskfs.slot.min.begin]
[linen.diskfs.slot.min.select.ok] path_id=1
[linen.diskfs.slot.min.stat.ok] size=4096 flags=0x3
[linen.diskfs.slot.min.write.ok] size=16
[linen.diskfs.slot.min.read.ok] size=16
[linen.diskfs.slot.min.match] ok=1
[linen.diskfs.slot.min.done] ok=1
```

## Runtime Gate
```
GATE_DIR=/tmp/gate_linen_min GATE_NVME=1 ./scripts/master_runtime_gate.sh --probe 900
```
Completes in ~120s (5 async PDX round-trips, each ~15-30s under QEMU NVMe).

## Files Changed
- `servers/linen/src/main.rs` — `run_linen_diskfs_slot_proof()` with async-aware blocking replies
- `sexos_build_spec.toml` — `rustflags = "--cfg linen_diskfs_slot_proof"`
- `docs/handoff/LINEN_DISKFS_SLOT_OBJECT_PROOF_V1.md` — this file

## Coverage Model
| Proof | Scope | Status |
|-------|-------|--------|
| `[sexfiles.disk.multi.linen.match] ok=1` | SexFiles-internal V2 deep stress (128B write/read) | ✅ |
| `[linen.diskfs.slot.min.match] ok=1` | Linen→SexFiles app boundary (16B write/read) | ✅ |
| Full 128B Linen→SexFiles | Impractical under QEMU NVMe+12PD timing | N/A |

## Zero Faults
```
#PF: 0   #GP: 0   panic: 0
```

## Next Phase
`QUIL_DISKFS_SLOT_OBJECT_PROOF_V1` — same 16B pattern for Quil path_id=2.
