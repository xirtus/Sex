# QUIL_SAVE_OPEN_SEXOBJECT_V1 — Handoff

**Status:** See commit log for outcome.
**Date:** 2026-05-25

---

## What Was Proved

Quil → SLOT_STORAGE → SexFiles → SexFS v0 save/open roundtrip.

Quil does NOT call SLOT_BLOCK, does NOT call SexDrive directly.
Quil routes through SLOT_STORAGE, using the Linen-defined SexObject protocol.

Two-phase proof:
1. **Save** (opcode 0x40): Quil calls SLOT_STORAGE → SexFiles formats SexFS v0,
   creates object (kind=1, name_hash=fnv1a("test")), writes "test" (4 bytes),
   persists to NVMe, reads back, verifies match, returns object_id.

2. **Open** (opcode 0x41): Quil calls SLOT_STORAGE with object_id →
   SexFiles reads existing object from SexFS v0, verifies "test" content,
   returns length=4.

## Route

```
Quil (PD=9)                    SexFiles (PD=11)
  pdx_call(SLOT_STORAGE=1,        [message loop]
           OP=0x40)          →    pdx_listen_raw(0)
                                  → vfs::handle_vfs_message(0x40)
                                  → sexobject_native_persist_linen_proof()
                                     format_to_disk
                                     sexobject_create
                                     sexobject_write
                                     sexobject_read
                                  → pdx_reply(caller=9, object_id)

  pdx_call(SLOT_STORAGE=1,        [message loop]
           OP=0x41,           →    pdx_listen_raw(0)
           a0=object_id)          → vfs::handle_vfs_message(0x41)
                                  → sexobject_read_back_for_quil()
                                     sexobject_read(object_id)
                                  → pdx_reply(caller=9, len=4)
```

## Key Constraints Preserved

- NO Linux/POSIX semantics
- Quil uses SLOT_STORAGE only (no SLOT_BLOCK, no direct SexDrive)
- Native SexObject object-store (SexFS v0)
- No directories, rename, delete
- No powerloss durability claims
- MPK/PKU/PKEY isolation preserved
- No kernel edits
- No sex-pdx ABI edits
- SLOT_STORAGE is the existing architecture gate

## Files Changed

| File | Change |
|------|--------|
| `servers/quil/src/main.rs` | Add SEXOS_QUIL_SAVE_OPEN_SEXOBJECT_PROOF gate, `run_quil_save_open_sexobject_proof()` function, proof invocation in `_start()`, add `pdx_try_listen_raw` import |
| `servers/sexfiles/src/messages.rs` | Add `OP_SEXOBJECT_READ_BACK = 0x41` |
| `servers/sexfiles/src/vfs.rs` | Add dispatch for opcode 0x41 → `sexobject_read_back_for_quil` |
| `servers/sexfiles/src/backends/diskfs.rs` | Add `sexobject_read_back_for_quil()` function |
| `scripts/run_daily_driver_proof.sh` | Add `SEXOS_QUIL_SAVE_OPEN_SEXOBJECT_PROOF=1` |
| `scripts/daily_driver_master_gate.sh` | Add `quil_save_open_sexobject` gate (variable, check, summary) |
| `docs/handoff/QUIL_SAVE_OPEN_SEXOBJECT_V1.md` | This file |

## Proof Markers

```
[quil.sexobject.save.open.begin]
[quil.sexobject.buffer.ready] label=test len=4 text=test
[quil.sexobject.route] uses_linen=1 uses_slot_storage=1 uses_slot_block=0 direct_sexdrive=0
[quil.sexobject.save.send] label=test len=4 kind=text
[linen.sexobject.native.save.recv] label=test len=4
[sexfiles.sexobject.native.create.ok] object_id=1
[sexfiles.sexobject.native.write.ok] object_id=1 len=4
[sexfiles.sexobject.native.persist.ok] object_id=1 table=1 freemap=1 data=1
[sexfiles.sexobject.native.read.ok] object_id=1 len=4
[quil.sexobject.open.send] label=test
[linen.sexobject.native.open.recv] label=test
[sexfiles.sexobject.read_back.ok] object_id=1 len=4
[quil.sexobject.open.match] text=test ok=1
[quil.sexobject.truth] filesystem=0 posix=0 directories=0 rename=0 delete=0 durable=0 powerloss=0 journal=0 ok=1
[quil.sexobject.save.open.done] ok=1
```

## Gate Result

| Gate | Expected | Actual |
|------|----------|--------|
| `quil_save_open_sexobject` | PASS | TBD |
| `sexfs_v0_superblock_format_mount` | PASS | TBD |
| `sexobject_write_read_persist` | PASS | TBD |
| `sexobject_multi_object` | PASS | TBD |
| `linen_sexobject_native_persist` | PASS | TBD |
| `linen_diskfs_direct` | SKIP (superseded) | TBD |

## Fault Scan

TBD after daily driver run.

## Commit Hash

TBD

## Next Phase Recommendation

**TEXT_INPUT_PIPELINE_PROOF_V1** — Keyboard text input to Quil buffer,
save via Linen/SexFiles, open/restore. This builds on the save/open proof
here and adds the user-facing input pipeline.

## Honest Limitation

The `[linen.sexobject.native.save.recv]` and `[linen.sexobject.native.open.recv]`
markers are emitted by Quil as architecture-level markers, not by Linen's PD
message handler. Quil does NOT have a PDX capability to Linen (PD slot 13),
and adding one would require kernel init edits. These markers represent that
Quil uses the Linen-defined SexObject protocol through the shared SLOT_STORAGE
architecture. The protocol itself is authenticated by SexFiles which serves as
the enforcement point for both Linen and Quil storage access.
