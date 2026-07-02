# QUIL_DISKFS_SLOT_OBJECT_PROOF_V1

## Date
2026-05-07

## Status
COMPLETE — Quil DiskFS app-boundary min proof implemented and passing.
SELECT 0x3E + V2 manifest are active. path_id=2 slot isolated from Linen (path_id=1)
and proof object (path_id=0).

## 1. Implementation Summary

| Attribute | Value |
|-----------|-------|
| Crate | `servers/quil/src/main.rs` |
| PD | 9 (domain_id=9 per init.rs spawn order) |
| Function | `run_quil_diskfs_slot_min_proof()` |
| Gate | `SEXOS_QUIL_DISKFS_SLOT_PROOF=1` (env var) |
| Pattern | Ported from Linen `run_linen_diskfs_slot_proof()` |
| Path | path_id=2 → `/disk/quil-object-v1` |
| Payload | 16B deterministic: `QUIL-SLOT-V1!!\0\x02` |
| Read strategy | 2×8B via reply path (not raw_reply-as-data) |
| Readiness | 64× `sched_yield()` cooperative wait before proof |

## 2. Proof Sequence (Option A: SELECT 0x3E, path_id=2)

```
1. [quil.diskfs.slot.min.begin]
2. 64× sched_yield() readiness wait
3. pdx_storage_call(OP_DISKFS_SELECT, 2, 0, 0)  → path_id=2
4. pdx_storage_call(OP_DISKFS_STAT, 0, 0, 0)     → verify object
5. Build 16B deterministic payload
6. pdx_storage_call(OP_DISKFS_WRITE, 0, lo, hi)  → write 16B
7. pdx_storage_call(OP_DISKFS_READ, 0, 8, 0)    → read first 8B
8. pdx_storage_call(OP_DISKFS_READ, 8, 8, 0)    → read last 8B
9. Byte-for-byte comparison
10. [quil.diskfs.slot.min.done] ok=1|0
```

## 3. DiskFS Slot Object Manifest (V2)

| Field | Value |
|-------|-------|
| path_id | 2 |
| Path | `/disk/quil-object-v1` |
| Hash | `0xaaf5c55ad6c063b5` |
| LBA | 2022-2029 (8 sectors, 4096 bytes) |
| Flags | 0x3 (READ\|WRITE) |
| Opcodes used | 0x3E (SELECT), 0x3B (STAT), 0x38 (WRITE), 0x39 (READ) |

## 4. Complete Marker Chain

Success:
```
[quil.diskfs.slot.min.begin]
[quil.diskfs.slot.min.select.ok] path_id=2
[quil.diskfs.slot.min.stat.ok] size=4096 flags=0x3
[quil.diskfs.slot.min.write.ok] size=16
[quil.diskfs.slot.min.read.ok] size=16
[quil.diskfs.slot.min.match] ok=1
[quil.diskfs.slot.min.done] ok=1
```

Error:
```
[quil.diskfs.slot.min.select.err] err=...
[quil.diskfs.slot.min.stat.err] err=...
[quil.diskfs.slot.min.write.err] err=...
[quil.diskfs.slot.min.read.err] err=...
[quil.diskfs.slot.min.match] ok=0 first_bad=... got=... expected=...
[quil.diskfs.slot.min.done] ok=0
```

## 5. Route Isolation Proof

Three slots operate independently:
- path_id=0: SexFiles internal proof (`/disk/sexfiles-proof-v1`)
- path_id=1: Linen app-boundary proof (`/disk/linen-object-v1`)
- path_id=2: Quil app-boundary proof (`/disk/quil-object-v1`)

Route audit (SEXFILES_ROUTE_AUDIT_ONLY=1) skips SexFiles-internal multi
object proof and delegates to app-boundary proofs only:
```
[sexfiles.disk.multi.skip] reason=route_audit
[sexfiles.vfs.enter]
[sexfiles.route.dispatch] op=0x3E/0x3B/0x38/0x39
[sexfiles.route.reply]
```

## 6. Runtime Gate Invocation

```
GATE_DIR=/tmp/gate_quil_min \
SEXOS_GATE_NVME=1 \
SEXFILES_ROUTE_AUDIT_ONLY=1 \
SEXOS_QUIL_DISKFS_SLOT_PROOF=1 \
./scripts/master_runtime_gate.sh --probe 900 --keep-log
```

Verification:
```
rg -n "quil\.diskfs\.slot\.min|sexfiles\.disk\.multi\.skip|sexfiles\.vfs\.enter|sexfiles\.route|#PF|#GP|panic" /tmp/gate_quil_min/serial.log
```

## 7. Safety Boundaries (All Hold)

| Boundary | Status |
|----------|--------|
| Quil uses SLOT_STORAGE only | VERIFIED — already has it |
| Quil does not receive SLOT_BLOCK | VERIFIED — not in capability set |
| Quil does not receive MemLend | VERIFIED — no sys_grant_mem_lend |
| Quil never calls SexDrive | VERIFIED — no BLOCK_* opcodes |
| No raw LBA exposure to Quil | VERIFIED — path_id only, never LBA |
| No broad Quil redesign | VERIFIED — proof function only |
| No kernel edits | VERIFIED |
| No sex-pdx ABI edits | VERIFIED |
| No protocol/opcode changes | VERIFIED |
| No DiskFS object map changes | VERIFIED |
| path_id=0 intact | VERIFIED — Quil uses path_id=2, never 0 |
| path_id=1 (Linen) untouched | VERIFIED — Quil uses path_id=2, never 1 |
| sexdisplay sole FB writer | VERIFIED |
| FB bounds checks preserved | VERIFIED |

## 8. Files Changed

| File | Change |
|------|--------|
| `servers/quil/src/main.rs` | Added DISKFS opcodes (0x38-0x3E), `run_quil_diskfs_slot_min_proof()`, gate flag, `sched_yield` import |
| `docs/handoff/QUIL_DISKFS_SLOT_OBJECT_PROOF_V1.md` | This doc — updated from plan to completion record |

### Files NOT touched (per scope)
- kernel/
- crates/sex-pdx/
- servers/sexfiles/
- servers/linen/
- servers/sexusb/
- servers/silk-shell/
- servers/sexdisplay/
