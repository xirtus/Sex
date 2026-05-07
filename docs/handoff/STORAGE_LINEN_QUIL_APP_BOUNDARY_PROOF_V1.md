# STORAGE_LINEN_QUIL_APP_BOUNDARY_PROOF_V1

## Scope
DiskFS V2 app-boundary persistence proof for Linen (path_id=1) and Quil (path_id=2), with SexFiles internal V2 proof preserved.

## Proven Marker Chains

### SexFiles Internal DiskFS V2
- `[sexfiles.disk.multi.linen.match] ok=1`
- `[sexfiles.disk.multi.quil.match] ok=1`
- `[sexfiles.disk.multi.proof_intact] first_byte=0x0`

### Linen App-Boundary (path_id=1)
- `[linen.diskfs.slot.min.begin]`
- `[linen.diskfs.slot.min.select.ok] path_id=1`
- `[linen.diskfs.slot.min.stat.ok] size=4096 flags=0x3`
- `[linen.diskfs.slot.min.write.ok] size=16`
- `[linen.diskfs.slot.min.read.ok] size=16`
- `[linen.diskfs.slot.min.match] ok=1`
- `[linen.diskfs.slot.min.done] ok=1`

### Quil App-Boundary (path_id=2)
- `[quil.diskfs.slot.min.begin]`
- `[quil.diskfs.slot.min.select.ok] path_id=2`
- `[quil.diskfs.slot.min.stat.ok] size=4096 flags=0x3`
- `[quil.diskfs.slot.min.write.ok] size=16`
- `[quil.diskfs.slot.min.read.ok] size=16`
- `[quil.diskfs.slot.min.match] ok=1`
- `[quil.diskfs.slot.min.done] ok=1`

### Route Isolation / Service Path
- `[sexfiles.disk.multi.skip] reason=route_audit`
- `[sexfiles.vfs.enter] ...`
- `[sexfiles.route.dispatch] op=0x3E/0x3B/0x38/0x39 ...`
- `[sexfiles.route.reply] op=0x3E/0x3B/0x38/0x39 ...`

## Critical Semantic
`SLOT_STORAGE` uses AsyncEnqueue semantics.

- `pdx_call(...)` raw return indicates enqueue status (ack), not operation result/data.
- DiskFS operation result/data is delivered via reply mailbox (`pdx_listen_raw` reply path, `type_id=0x1`).
- Read payload validation must decode reply values, not assume `pdx_call` raw value is data.

## Fault Status
From proof runs used for this freeze:
- `#PF = 0`
- `#GP = 0`
- `panic = 0`

## Gate Command Used
```bash
GATE_DIR=/tmp/gate_quil_min_v2 \
SEXOS_GATE_NVME=1 \
SEXFILES_ROUTE_AUDIT_ONLY=1 \
SEXOS_QUIL_DISKFS_SLOT_PROOF=1 \
./scripts/master_runtime_gate.sh --probe 900 --keep-log
```
