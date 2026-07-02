# LINEN_SEXOBJECT_NATIVE_PERSIST_V1 — Handoff

**Status:** COMPLETE. `linen_sexobject_native_persist` PASS.
**Commit:** `6ff30f16`
**Date:** 2026-05-25

---

## What Was Proved

Linen → SexFiles SLOT_STORAGE (opcode 0x40) → SexFS v0 NVMe → reply → Linen.

Required markers all present in proof log:
```
[linen.sexobject.native.begin]
[linen.sexobject.native.route] uses_slot_storage=1 uses_slot_block=0 direct_sexdrive=0
[linen.sexobject.native.save.send] label=test len=4 kind=text
[sexfiles.sexobject.native.create.ok] object_id=1
[sexfiles.sexobject.native.write.ok] object_id=1 len=4
[sexfiles.sexobject.native.persist.ok] object_id=1 table=1 freemap=1 data=1
[sexfiles.sexobject.native.read.ok] object_id=1 len=4
[linen.sexobject.native.read.match] label=test text=test ok=1
[linen.sexobject.native.truth] filesystem=0 posix=0 directories=0 rename=0 delete=0 durable=0 powerloss=0 journal=0 ok=1
[linen.sexobject.native.done] ok=1
```

## Gate Results

| Gate | Result |
|------|--------|
| `sexfs_v0_superblock_format_mount` | PASS |
| `sexobject_table_persist` | PASS |
| `sexobject_table_extent_alloc` | PASS |
| `sexobject_extent_write_full_block` | PASS |
| `sexobject_write_read_persist` | PASS |
| `sexobject_multi_object` | PASS |
| `linen_diskfs_direct` | SKIP (superseded — correct) |
| `linen_sexfiles_100_current_tier_release` | SKIP (not triggered in this profile) |
| `linen_sexobject_native_persist` | **PASS** |

## Root Cause of Prior Failures

**WAIT_YIELDS=16384 was calibrated for ~0.5ms/yield. Actual yield cost was ~370ms.**

The `run_linen_sexobject_native_persist_proof()` retry loop used:
- `WAIT_YIELDS=16384` — one wait loop = 16384 sys_yield calls
- System delivered only ~736 linen yields in 270s (= ~25 yields/sec at idle)
- 16384 yields would require ~6000 seconds — never completed

Investigation path:
1. Confirmed kernel enqueue path (ipc.rs `traverse_edge` → `AsyncEnqueue { ring }`) correctly enqueues `IpcCall{func_id=0x40}` to sexfiles' message_ring.
2. Confirmed dequeue path (syscall 28 slot=0 → `current_pd.message_ring.dequeue()`) correctly returns `func_id` as `type_id`.
3. Confirmed SLOT_STORAGE → `CapabilityData::Domain(sexfiles_id)` correctly set in init.rs.
4. Confirmed DOMAIN_REGISTRY and core_local use same PD pointer (same `message_ring`).
5. Found in log: `[sexfiles.trampoline.listen.enter]` printed once at line 93510 but `[sexfiles.trampoline.after_listen]` never printed — `pdx_listen_raw(0)` never returns.
6. Confirmed linen yields: only 736 total in 270s log. At WAIT_YIELDS=16384, linen never completes even ONE attempt's wait loop.
7. After sexfiles enters message loop (t≈252.7s), only 17.3s remaining. In those 17.3s, linen gets ~428 yields — still only 2.6% of WAIT_YIELDS=16384.

## Fix

**`servers/linen/src/main.rs`** (single-line change):
```rust
const WAIT_YIELDS: u64 = 128;  // was 16384
```

**`scripts/run_daily_driver_proof.sh`**:
```sh
PROBE_SECONDS=300  // was 270
```

With WAIT_YIELDS=128:
- At idle yield rate ~25 yields/sec: one wait loop = 128/25 = 5.1s
- Sexfiles processes 4 NVMe ops in ~2s < 5.1s → reply arrives before linen retries
- 300s budget: sexfiles startup ~253s + 47s slack for linen retry + proof

## Architecture

```
linen (pd=7)                    sexfiles (pd=11)
  pdx_call(SLOT_STORAGE=1,        [message loop]
           OP=0x40)          →    pdx_listen_raw(0)
                                  → vfs::handle_vfs_message(0x40)
                                  → sexobject_native_persist_linen_proof()
                                     format_to_disk (NVMe write)
                                     sexobject_create (NVMe write)
                                     sexobject_write (NVMe write)
                                     sexobject_read (NVMe read)
                                  → pdx_reply(caller=7, object_id)
  pdx_try_listen_raw(0)       ←    incoming_replies[linen] += reply
  type_id=0x1, arg0=object_id
  → print success markers
```

Key constraints preserved:
- NO Linux/POSIX semantics
- Linen uses SLOT_STORAGE only (no SLOT_BLOCK, no direct SexDrive)
- Native SexObject object-store (SexFS v0)
- No directories, rename, delete
- No powerloss durability claims
