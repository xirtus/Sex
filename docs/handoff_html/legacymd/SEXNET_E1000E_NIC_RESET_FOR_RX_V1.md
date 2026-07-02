# SEXNET_E1000E_NIC_RESET_FOR_RX_V1

## Root Cause Hypothesis

HAL diagnostic (source=2) fully enables e1000e RX/TX with its own ring addresses
at lines 459-624 of `kernel/src/hal/pci.rs`:
- Writes RCTL with EN=1 at line 462-463
- Writes TCTL with EN=1 at line 615-616
- Writes IMS with RX/TX/LSC interrupt bits at line 489
- Writes RXDCTL.ENABLE, SRRCTL, RXCSUM at lines 480-489

When sexnet (source=3) takes over NIC ownership, it:
1. Reads current register state (HAL's enabled configuration)
2. Swaps ring addresses (RDBAL/TDBAL) without device reset
3. Enables RCTL/TCTL with sexnet's rings

**Root cause**: The e1000e internal state machine (descriptor fetch engine,
RX FIFO, DMA engine) retains stale state from HAL's prior enable. Swapping
ring addresses without a full device reset leaves the RX datapath in an
inconsistent state — TX works because it's push-driven; RX fails because
the descriptor fetch engine/queue controls are latched to HAL's old state.

## Current Register Sequence (sexnet permanent RX)

```
RCTL.EN = 0
RDBAL = perm_desc_phys_lo
RDBAH = perm_desc_phys_hi
RDLEN = 128
RDH   = 0
RDT   = 7
SRRCTL(0) = 0x0002
RCTL.EN = 1 (with UPE|MPE|BAM|SECRC)
```

**Missing**: No CTRL.RST, no IMC write, no RXDCTL write, no link verification.

## Proposed Reset Sequence

Before sexnet permanent ring programming:

1. Disable RX: RCTL.EN = 0, readback verify
2. Disable TX: TCTL.EN = 0, readback verify
3. Mask interrupts: IMC(0x00D8) = 0xFFFF_FFFF
4. CTRL.RST(0x0000, bit 26) = 1
5. Bounded poll (max 1M iterations) until CTRL.RST bit 26 clears
6. Read MAC post-reset: RAL(0x5400), RAH(0x5404) — auto-load from EEPROM
7. Set RXDCTL(0).ENABLE(0x2828) and TXDCTL(0).ENABLE(0x3828)
8. Bounded link poll: STATUS(0x0008) LU bits 10 (e1000e) or 1 (e1000)
9. Continue with existing permanent ring programming

## STOP Boundaries

- No kernel edits
- No sex-pdx/global ABI edits
- No DMA memory ownership model change
- No scheduler/PKRU/time changes
- No NIC driver rewrite
- No HAL deletion
- No HTTP/browser/socket API

## Rollback Plan

1. Restore from backup: `/tmp/sexnet_e1000e_reset_backup/`
2. git checkout servers/sexnet/src/main.rs
3. Reset gate removal: `git checkout scripts/daily_driver_master_gate.sh`

## Proof Markers

```
[sexnet.nic.reset.begin] ok=1
[sexnet.nic.reset.rx.disable] ok=1
[sexnet.nic.reset.tx.disable] ok=1
[sexnet.nic.reset.irq.mask] ok=1
[sexnet.nic.reset.ctrl.rst.write] ok=1
[sexnet.nic.reset.ctrl.rst.poll] cleared=1 polls=N ok=1
[sexnet.nic.reset.mac.program] ral=... rah=... valid=N ok=1
[sexnet.nic.reset.queue.enable] rxdctl=1 txdctl=1 ok=1
[sexnet.nic.reset.status] link_up=N ok=1
[sexnet.nic.reset.proof.done] ok=1
```

## Proof Run Result (2026-05-19)

Reset sequence implemented and proven:
```
[sexnet.nic.reset.begin] ok=1
[sexnet.nic.reset.rx.disable] ok=1
[sexnet.nic.reset.tx.disable] ok=1
[sexnet.nic.reset.irq.mask] ok=1
[sexnet.nic.reset.ctrl.rst.write] ok=1
[sexnet.nic.reset.ctrl.rst.poll] cleared=1 polls=0 ok=1
[sexnet.nic.reset.mac.program] ral=0x12005452 rah=0x80005634 valid=1 ok=1
[sexnet.nic.reset.queue.enable] rxdctl=1 txdctl=1 ok=1
[sexnet.nic.reset.status] link_up=1 ok=1
[sexnet.nic.reset.proof.done] ok=1
```

All 10 reset markers ok=1. 251 gates PASS, 0 faults.

HOWEVER: sexnet source=3 RX still shows `dd_set=0`, `frames_rx=0` after reset.
SYN TX dd=1 confirmed. No SYN-ACK received. TCP stuck at SYN_SENT.

## Additional Root Cause: Cache Coherency (UC vs WB)

The CTRL.RST fix is necessary but INSUFFICIENT. Additional root cause found:

### HAL (source=2) ring mapping — UC (Uncacheable) — WORKS
`kernel/src/hal/pci.rs` line 214:
```rust
let uc_flags = PageTableFlags::PRESENT
    | PageTableFlags::WRITABLE
    | PageTableFlags::NO_CACHE         // ← UC
    | PageTableFlags::WRITE_THROUGH    // ← UC
    | PageTableFlags::NO_EXECUTE;
```

### Sexnet (source=3) ring mapping — WB (Write-Back) — FAILS
`kernel/src/syscalls/mod.rs` line 275 (syscall 30, MAP_MEMORY):
```rust
let flags = PageTableFlags::PRESENT
    | PageTableFlags::WRITABLE
    | PageTableFlags::USER_ACCESSIBLE;
// Missing: NO_CACHE | WRITE_THROUGH
```

### Why TX works but RX fails:
- **TX**: CPU writes descriptors → WB cache → eventually flushed → e1000e DMA reads OK
- **RX**: e1000e DMA writes descriptors (sets DD=1) → WB cache line may be stale →
  CPU `read_volatile` hits stale cache → DD always reads 0 → no packets seen

### Fix Required:
Add `NO_CACHE | WRITE_THROUGH` to syscall 30 (MAP_MEMORY) flags in
`kernel/src/syscalls/mod.rs` line 275-277, changing:
```rust
let flags = PageTableFlags::PRESENT
    | PageTableFlags::WRITABLE
    | PageTableFlags::USER_ACCESSIBLE;
```
To:
```rust
let flags = PageTableFlags::PRESENT
    | PageTableFlags::WRITABLE
    | PageTableFlags::USER_ACCESSIBLE
    | PageTableFlags::NO_CACHE
    | PageTableFlags::WRITE_THROUGH;
```

## Classification

**STOP FIRST** — The e1000e CTRL.RST is implemented and proven, but RX remains blocked
by a cache coherency bug in the kernel's MAP_MEMORY syscall (WB instead of UC).
Kernel edit required per STOP FIRST boundary.
