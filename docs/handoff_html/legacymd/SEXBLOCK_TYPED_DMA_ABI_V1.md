# SEXBLOCK_TYPED_DMA_ABI_V1

## 1. ABI Constants (source: `crates/sex-pdx/src/lib.rs`)

### Commands (pdx_call opcode, decoded as msg.type_id by sexdrive)
| Constant | Value | Purpose |
|----------|-------|---------|
| `BLOCK_READ` | 1 | Read sectors from block device |
| `BLOCK_WRITE` | 2 | Write sectors to block device |
| `BLOCK_SYNC` | 3 | Flush/barrier (no data transfer, arg0/arg1/arg2 ignored) |

### Status Codes (pdx_reply value)
| Constant | Value | Meaning |
|----------|-------|---------|
| `BLOCK_OK` | 0 | Success |
| `BLOCK_ERR_BAD_CMD` | 1 | Unknown/unsupported command |
| `BLOCK_ERR_BAD_LEN` | 2 | Transfer size out of bounds or offset unaligned |
| `BLOCK_ERR_BAD_CAP` | 3 | Invalid/missing buffer capability token |
| `BLOCK_ERR_NO_DEVICE` | 4 | No real block device backend (honest refusal) |
| `BLOCK_ERR_TIMEOUT` | 5 | Operation timed out |

### Protocol Bounds
| Constant | Value | Meaning |
|----------|-------|---------|
| `BLOCK_SECTOR_SIZE` | 512 | Minimum alignment unit (bytes) |
| `BLOCK_MAX_XFER` | 4096 | Max bytes per transfer (one page) |

### Slot
| Constant | Value |
|----------|-------|
| `SLOT_BLOCK` | 15 |

## 2. Arg Packing (pdx_call encoding)

```
pdx_call(SLOT_BLOCK, cmd, offset, size, buffer_cap)
         rdi=15     rsi   rdx    r10   r8
```

| Register | Field | Meaning |
|----------|-------|---------|
| rdi | slot | SLOT_BLOCK = 15 |
| rsi | cmd | BLOCK_READ/WRITE/SYNC |
| rdx | offset | Byte offset in block device (must be sector-aligned) |
| r10 | size | Transfer size in bytes (≤ BLOCK_MAX_XFER) |
| r8 | buffer_cap | Buffer capability token (0 = no DMA buffer yet) |

### Reply (pdx_reply encoding)
```
pdx_reply(caller_pd, status)
          rdi       rsi
```

| Register | Field | Meaning |
|----------|-------|---------|
| rdi | caller_pd | PD that sent the request |
| rsi | status | Block status code (BLOCK_OK=0, or error code) |

## 3. Route Flow
```
sexfiles.diskfs.typed.call
  → diskfs_block_read/write/sync()
    → pdx_call(SLOT_BLOCK=15, BLOCK_READ, offset, size, buffer_cap)
      → [kernel capability check: sexfiles→sexdrive at slot 15]
        → sexdrive.block.typed.recv (pdx_try_listen_raw(0))
          → [sexblock.abi.request.decode] match on cmd
            → dispatch: BLOCK_READ|BLOCK_WRITE|BLOCK_SYNC
              → bounds check (size ≤ BLOCK_MAX_XFER, offset % BLOCK_SECTOR_SIZE == 0)
              → ERR_NO_DEVICE (no real NVMe/AHCI backend)
          ← [sexblock.abi.reply.encode] pdx_reply(caller_pd, status)
      ← [kernel routes reply back to sexfiles]
    ← (kernel_status, block_status)
  ← sexfiles.diskfs.typed.reply
```

## 4. Honest Status Contract
- **BLOCK_READ/WRITE/SYNC** → `ERR_NO_DEVICE` — no real NVMe/AHCI driver exists
- **Unknown command** (anything not 1/2/3) → `ERR_BAD_CMD`
- **Oversized transfer** (size > 4096) → `ERR_BAD_LEN`
- **Unaligned offset** (offset % 512 != 0) → `ERR_BAD_LEN`
- **No fake success**: sexdrive never returns BLOCK_OK for read/write until a real backend is wired.

## 5. Files Changed
| File | Change |
|------|--------|
| `crates/sex-pdx/src/lib.rs` | +14 lines: block cmd/status/bounds constants |
| `servers/sexfiles/src/pdx.rs` | re-exports: trimmed to SLOT_BLOCK + 3 status codes |
| `servers/sexfiles/src/backends/diskfs.rs` | +50 lines: `diskfs_block_read/write/sync` typed wrappers |
| `servers/sexfiles/src/proof.rs` | +55 lines: typed route proof (read/write/sync/bad_cmd/bad_len/unaligned) |
| `apps/sexdrive/src/main.rs` | +20 lines: typed command decode + bounds check + honest ERR_NO_DEVICE |
| `docs/handoff/SEXBLOCK_TYPED_DMA_ABI_V1.md` | new: this document |

## 6. Whether Real Read Is Still Blocked
**YES — real block read is still blocked.**
- `BLOCK_READ`, `BLOCK_WRITE`, `BLOCK_SYNC` all return `ERR_NO_DEVICE`.
- Missing: NVMe/AHCI PCI driver, DMA engine, PRP/SGL scatter-gather lists.
- The typed ABI contract is proven correct (decode, bounds check, honest status).
- Next step: wire a real NVMe/AHCI backend behind BLOCK_READ/BLOCK_WRITE.

## 7. Build Result
```bash
bash build_payload.sh
# → sexfiles: 0 warnings, sexdrive: 1 pre-existing warning (unused unsafe)
# → kernel: pre-existing warnings only
# → ✅ All PDX modules staged
```

## 8. Runtime Proof Markers
Expected trace (with SEXOS_SEXFILES_REAL_BLOCK_PROOF=1):

```
[sexfiles.diskfs.typed.call] cmd=BLOCK_READ offset=0x0 size=512 buf_cap=0x0
[sexfiles.diskfs.call] slot=15 opcode=0x1 arg0=0x0 arg1=0x200 arg2=0x0
[sexdrive.block.typed.recv] cmd=1 offset=0x0 size=512 buf_cap=0x0 caller=<sexfiles_pd>
[sexdrive.block.typed] cmd=1 ERR_NO_DEVICE honest=no_nvme_ahci_backend
[sexblock.abi.reply.encode] caller=<sexfiles_pd> status=4
[sexdrive.block.typed.reply] cmd=1 caller=<sexfiles_pd> status=4
[sexfiles.diskfs.reply] status=0x0 value=0x4
[sexfiles.diskfs.typed.reply] cmd=BLOCK_READ status=4
[sexfiles.block.proof.typed_read] status=4 expected=ERR_NO_DEVICE(4)
```

### Grep command
```bash
SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log \
  | grep -E 'sexfiles\.diskfs\.(typed\.(call|reply)|block\.proof\.(typed|bad|unaligned))|sexdrive\.block\.typed|sexblock\.abi'
```

## 9. Next Prompt Recommendation
`SEXBLOCK_REAL_NVME_BACKEND_V1` — wire a minimal NVMe PCI probe + admin queue behind BLOCK_READ to make the first real sector read succeed. Requires STOP FIRST for PCI BAR mapping, MSI-X, and PRP list construction.
