# SEXFILES_QUIL_REBOOT_PERSISTENCE_PROOF_V1

- date: 2026-05-06
- git baseline: fc8c04a271e046210ef59c64751855ce74003ff6
- proof gate: `--cfg sexfiles_quil_persistence_proof` (build-time)
- runtime gate: `SEXOS_SEXFILES_BOOT_PROOF=1`
- runtime gate result: **GREEN_MASTER**

## Goal

Prove Quil can save/load through SexFiles using the locked buffer/object protocol,
with an honest persistence boundary.

## Actual Persistence Level Proven

**In-memory save/load roundtrip (same boot).** Quil initializes its text buffer
(240 bytes), saves to sexfiles RamFS, clears local buffer, loads from RamFS,
and verifies byte-for-byte match.

Real reboot persistence is NOT claimed — RamFS is volatile (in-memory).
Disk-backed persistence requires DiskFS backend (stub) and journal/replay
scaffolding (see blocker below).

## PDX Architecture Discovery

**All inter-PD calls are fire-and-forget.** The kernel's `safe_pdx_call` →
`traverse_edge` → `AsyncEnqueue` returns `Ok(0u64)` always — the caller never
receives the server's reply through the pdx_call syscall. Replies go to a
separate `incoming_replies` queue, surfaced via `pdx_listen_raw(0)` as
synthetic messages with `type_id=0x1` and `arg0=reply_value`.

This was confirmed by adding VFS-side debug markers:
- sexfiles VFS returns `handle=1` from `OP_RAMFS_OPEN`
- sexfiles trampoline calls `pdx_reply(caller=9, reply=1)`
- Quil's `pdx_call` return value is always `(0, 0)`

### Fix: Synchronous PDX Call Wrapper

Added `pdx_call_and_reply()` and `pdx_storage_call()` to Quil that:
1. Send the PDX call (fire)
2. Block on `pdx_listen_raw(0)` for the matching reply (`type_id == 0x1`)
3. Skip non-reply messages that arrive before the reply
4. Return `(status, reply_value)` as if it were synchronous

This matches the existing listen syscall design (syscall 28 checks
`incoming_replies` queue first, returns `type_id=0x1` with `arg0=reply_value`).

## Files Changed (This Task)

| File | Change |
|------|--------|
| `servers/sexfiles/src/vfs.rs` | Mask flag byte from arg2 in `OP_RAMFS_OPEN` name unpacking (`arg2 & !(0xFFu64 << 24)`) — fixes name mismatch between save (O_CREATE flag in arg2 byte 3) and load (no flags) |
| `servers/quil/src/main.rs` | Added `pdx_call_and_reply()` + `pdx_storage_call()` synchronous wrappers; rewrote `quil_save()` and `quil_load()` to use them; added persistence proof block gated by `cfg!(sexfiles_quil_persistence_proof)` |
| `scripts/sexos_build_trace.sh` | Added optional `rustflags` key to `cargo_manifest` action (reads `rustflags = "..."` from stage spec) |
| `sexos_build_spec.toml` | Added `rustflags = "--cfg sexfiles_quil_persistence_proof"` to Quil build stage |

**Total: 4 files, ~140 net lines added.**

## Proof Markers (Serial Log)

```
[quil.sexfiles.proof.start]
[quil.sexfiles.proof.open]              ← OP_RAMFS_OPEN succeeded (sync reply)
[quil.sexfiles.proof.write] ok          ← 240 bytes written (30 chunks × 8 bytes)
[quil.sexfiles.proof.read] ok           ← 240 bytes read back
[quil.sexfiles.proof.match] 240 bytes   ← byte-for-byte match verified
[quil.sexfiles.proof.deny] invalid_handle error=-1  ← invalid handle correctly returns ERR_INVALID_HANDLE
[quil.sexfiles.proof.done]
```

### Missing Marker

- `[quil.sexfiles.proof.replay_match]` — NOT PRESENT. Disk persistence / journal
  replay not yet implemented. See blocker below.

## Build / Runtime Result

```
cargo check -p quil    → PASS (pre-existing warnings only)
cargo check -p sexfiles → PASS (clean)
./scripts/entrypoint_build.sh → PASS (ISO with persistence proof compiled in)

SEXOS_SEXFILES_BOOT_PROOF=1 ./scripts/master_runtime_gate.sh --probe 30 --keep-log
  BUILD_GATE       PASS
  SPAWN_GATE       PASS
  CLOCK_GATE       PASS
  SCHED_GATE       PASS
  FAULT_GATE       PASS
  SEXFILES_GATE    PASS
  FINAL_SCORE      GREEN_MASTER
```

## Exact Blocker to True Reboot Persistence

1. **RamFS is volatile.** Data lives in a `static RamFs` with `Vec<FileEntry>` —
   lost on power loss. No write-back to disk.

2. **DiskFS is a stub.** All `DiskFs` operations return `ERR_NOT_FOUND`. No
   AHCI/NVMe driver integration, no block read/write path.

3. **No journal/replay.** The `sexfiles/src/proof.rs` has stub functions for
   `run_diskfs_object_table_proofs`, `run_sexfiles_journal_proofs`,
   `run_sexfiles_replay_proofs`, and `run_sexfiles_cap_record_proofs` but
   these require the DiskFS backend to be implemented first.

4. **PDX async limitation.** The `pdx_call_and_reply` wrapper works for
   boot-time proof but has a fundamental issue during runtime: it calls
   `pdx_listen_raw(0)` which can consume non-reply messages (HID events,
   pings). A proper fix requires either:
   - A dedicated reply slot (kernel change — STOP FIRST)
   - Making `safe_pdx_call` synchronous in the kernel (kernel change — STOP FIRST)
   - A separate listen ring for replies

## What Was Intentionally NOT Done

- No kernel edits (beyond boot deploy spawn addition from previous task)
- No sex-pdx ABI edits
- No Quil editor redesign
- No POSIX paths
- No disk persistence claim
- No journal/replay implementation
- No real reboot proof (requires DiskFS + journal)

## Dependencies

- SEXFILES_BOOT_DEPLOY_V1 (completed — sexfiles spawned as PD 11)
- SEXFILES_RAMFS_CONTRACT_LOCK_V1 (completed — RamFS backend)
- SEXFILES_NAMESPACE_CAPS_V1 (completed — per-file owner PD)
- SLOT_STORAGE grant to Quil (exists in kernel init.rs)
- SLOT_STORAGE already defined in sex-pdx (slot 1)

## Next Steps

1. `SEXFILES_DISKFS_BACKEND_V1` — implement DiskFS backend with block device access
2. `SEXFILES_JOURNAL_REPLAY_V1` — add journal and replay for crash safety
3. `SEXFILES_PDX_REPLY_SLOT_V1` — dedicated reply slot for synchronous PDX calls
   (requires STOP FIRST for kernel/ABI change)
