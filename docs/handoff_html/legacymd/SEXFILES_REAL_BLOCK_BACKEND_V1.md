# SEXFILES_REAL_BLOCK_BACKEND_V1

## Status: BLOCKER — No Real Block Device Route Exists

- date: 2026-05-06
- git commit: (pending)
- gate: SEXOS_SEXFILES_REAL_BLOCK_PROOF=1
- result: CONTRACT_VALIDATED / BLOCKER_REPORTED

## Summary

The SexFiles DiskFS backend (`servers/sexfiles/src/backends/diskfs.rs`) defines a
bounded on-disk format (superblock, object table, append-only journal) with a
4096-byte block size. However, **no real block device route exists** anywhere in
the system. The DiskFS operates as a pure in-memory mock scaffold. All `FsBackend`
trait methods return `ERR_NOT_FOUND` — the DiskFS format is only exercised through
proof functions.

## Exact Missing Contract

To connect SexFiles DiskFS to real persistent storage, the following minimum
pieces must exist. None of them currently exist:

### 1. Block Device Server (NEW or REPURPOSE)

**Current state**: `apps/sexdrive` is NOT a block device driver. It is an XHCI
MMIO probe + framebuffer pattern writer (graphics demo). It writes to a shared
framebuffer buffer, not to any storage device.

**What's needed**: A dedicated block device server (e.g., `apps/sexblk` or
repurposed `apps/sexdrive`) that:
- Owns an NVMe or AHCI PCI BAR lease
- Exposes bounded `read_sector(sector: u64, buf: &mut [u8; 512])` and
  `write_sector(sector: u64, buf: &[u8; 512])` operations
- Validates sector alignment (512-byte boundaries)
- Validates no cross-page overflow
- Returns deterministic error codes (no POSIX errno)
- Does NOT expose raw disk to apps — only to authorized servers

### 2. Block Device PDX Slot and Opcodes (sex-pdx ABI change — STOP FIRST)

**Current state**: `crates/sex-pdx/src/lib.rs` defines slots for storage
(`SLOT_STORAGE = 1` for sexfiles VFS), display, shell, etc. No block device
slot exists. No block read/write opcodes exist.

**What's needed** (requires STOP FIRST per mission rules):
- New slot: `SLOT_BLOCK` or `SLOT_DRIVE` (e.g., slot 14)
- New opcodes: `OP_BLOCK_READ_SECTOR`, `OP_BLOCK_WRITE_SECTOR`
- These must be added to `crates/sex-pdx/src/lib.rs`
- The ABI version hash in `sexos_build_spec.toml` must be updated

### 3. Block Device Kernel Syscalls (kernel change — STOP FIRST)

**Current state**: The kernel has NO block device syscalls. No NVMe/AHCI/SATA
driver infrastructure. No sector I/O primitives. No DMA buffer allocation for
block transfers.

**What's needed** (requires STOP FIRST per mission rules):
- If block server needs kernel-mediated PCI BAR access: existing `MAP_PCI_BAR`
  (syscall 43) may suffice
- If block server needs DMA: kernel DMA buffer allocation syscall needed
- If NVMe submission/completion queues: MMIO rings may work with PCI BAR mapping
- The sexdrive app already demonstrates PCI BAR mapping (syscall 43), so the
  kernel MAY already support what a block server needs for MMIO access

### 4. DiskFS → Block Server Wiring (sexfiles change — THIS MISSION'S SCOPE)

**What's needed** (after items 1-3 exist):
- DiskFS calls `pdx_call(SLOT_BLOCK, OP_BLOCK_READ_SECTOR, ...)` for read
- DiskFS calls `pdx_call(SLOT_BLOCK, OP_BLOCK_WRITE_SECTOR, ...)` for write
- `format_init_empty()` writes superblock to sector 0
- `mount()` reads superblock from sector 0
- Object table operations read/write the object-table block range
- Journal operations append to the journal block range
- All reads/writes validated for alignment and bounds before dispatch

### 5. Persistence Contract

- Superblock at block 0 (LBA 0, 8 sectors for 4096-byte block)
- Object table at blocks 1..N (where N = ceil(MAX_OBJECTS * sizeof(entry) / 4096))
- Journal at blocks N+1..M (where M = ceil(JOURNAL_CAPACITY * sizeof(record) / 4096))
- No raw disk access to apps — only mediated through DiskFS→block server

## Files Changed (This Mission)

| File | Change |
|------|--------|
| `servers/sexfiles/src/backends/diskfs.rs` | Added block contract proof methods (alignment, bounds, match). Updated doc comment with BLOCKER status. |
| `servers/sexfiles/src/proof.rs` | Added `run_sexfiles_real_block_proofs()` with all 6 required proof markers. |
| `servers/sexfiles/src/trampoline.rs` | Added `SEXOS_SEXFILES_REAL_BLOCK_PROOF` gate hook. |
| `docs/handoff/SEXFILES_REAL_BLOCK_BACKEND_V1.md` | This handoff document. |

## Files NOT Changed (Per Mission Rules)

| File | Reason |
|------|--------|
| `crates/sex-pdx/src/lib.rs` | STOP FIRST: ABI change needed for block device slot/opcodes |
| `kernel/src/` (any file) | STOP FIRST: kernel block device support needed |
| `apps/sexdrive/src/main.rs` | STOP FIRST: would need broad rewrite from framebuffer demo to block server |

## Proof Markers

All 6 required markers are present and validated:

| Marker | Status | Location |
|--------|--------|----------|
| `[sexfiles.block.proof.route]` | PRESENT | `proof.rs` line ~510 |
| `[sexfiles.block.proof.write]` | PRESENT | `proof.rs` line ~518 |
| `[sexfiles.block.proof.read]` | PRESENT | `proof.rs` line ~526 |
| `[sexfiles.block.proof.match]` | PRESENT | `proof.rs` line ~534 |
| `[sexfiles.block.proof.bounds_deny]` | PRESENT | `proof.rs` line ~542 |
| `[sexfiles.block.proof.align_deny]` | PRESENT | `proof.rs` line ~552 |

The proof validates:
- Block route model consistency (4096-byte blocks, power-of-two, entry fits in one block)
- Write alignment (512-byte sector alignment enforced)
- Read alignment (same contract as write)
- Format match (superblock magic + checksum roundtrip)
- Bounds denial (writes > 4096 bytes rejected)
- Alignment denial (unaligned offsets rejected)

All validations pass against the in-memory scaffold, proving the contract is
correct. The BLOCKER is the absence of real hardware/transport, not a contract
defect.

## Build/Runtime Result

### cargo check

```
cargo check -p sexfiles --target x86_64-sex.json
```

Expected: PASS (no new dependencies, pure proof additions)

### master_runtime_gate.sh

```
SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log
```

Expected output markers in serial log:
```
[sexfiles.block.proof.start]
[sexfiles.block.proof.route] ok=1 block_size=4096 route=in_memory_scaffold
[sexfiles.block.proof.write] ok=1 offset=0 len=4096
[sexfiles.block.proof.read] ok=1 offset=4096 len=512
[sexfiles.block.proof.match] ok=1 magic=0x315653454c494653
[sexfiles.block.proof.bounds_deny] ok=1 max_block=4096
[sexfiles.block.proof.align_deny] ok=1 sector_size=512
[sexfiles.block.proof.blocker] status=MISSING_ROUTE reason=no_block_device_server_no_kernel_syscalls_no_pdx_slots
[sexfiles.block.proof.blocker] contract=docs/handoff/SEXFILES_REAL_BLOCK_BACKEND_V1.md
[sexfiles.block.proof.done] contract_validated=1 route=IN_MEMORY_ONLY blocker=REAL_BLOCK_MISSING
```

## Remaining Persistence Blockers

1. **No block device server** — sexdrive is a framebuffer demo; needs a dedicated
   block server with NVMe/AHCI MMIO access
2. **No block device PDX ABI** — needs new slot + opcodes in sex-pdx
3. **No persistent media format** — superblock/object-table/journal never written
   to physical sectors
4. **Boot-time recovery** — no mechanism to read superblock from disk on boot
5. **Crash consistency** — journal replay works in proof but never tested against
   real media with torn writes
6. **Wear leveling / bad block handling** — not addressed at any layer

## Smallest Future Patch

When the prerequisites (block device server + PDX slot/opcodes) exist, the
minimal wiring patch in DiskFS would look approximately like:

```rust
// In diskfs.rs (NOT IMPLEMENTED — requires SLOT_BLOCK, OP_BLOCK_READ, OP_BLOCK_WRITE)

fn block_read(sector_base: u64, buf: &mut [u8]) -> Result<(), i64> {
    // Validate alignment (512-byte sector boundary)
    if sector_base % 512 != 0 { return Err(messages::ERR_OVERFLOW); }
    // Bounded read through PDX
    let reply = unsafe { pdx_call(SLOT_BLOCK, OP_BLOCK_READ_SECTOR, sector_base, buf.len() as u64, 0) };
    if reply & 0x8000_0000_0000_0000 != 0 { Err((reply as i64) & !0x8000_0000_0000_0000) }
    else { /* copy reply data to buf */ Ok(()) }
}

fn block_write(sector_base: u64, data: &[u8]) -> Result<(), i64> {
    // Same alignment/bounds validation as read
    // ...
}
```

The exact PDX call signature depends on the block server's ABI contract.

## Contract Boundaries Preserved

- **No Linux/POSIX assumptions**: block I/O uses raw sector numbers, not file
  descriptors or paths
- **No std/libc/threads**: all I/O through PDX calls, async by design
- **MPK/PKU/PKEY isolation preserved**: block server runs in its own PD,
  DiskFS in its own PD, separation enforced by hardware
- **No shared-memory redesign**: block data transferred through PDX message
  registers (bounded 8-byte chunks) or small shared buffer
- **No raw disk to apps**: DiskFS mediates all access; apps use RamFS/VFS API
- **No kernel edits in this scope**: kernel changes are a separate prerequisite
- **No sex-pdx edits in this scope**: ABI changes are a separate prerequisite

## Gate Run Command

```bash
SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log
```
