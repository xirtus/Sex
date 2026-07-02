# SEXDRIVE_BACKEND_REALITY_AUDIT_V1

- date: 2026-05-07
- git commit: (current)
- result: backend status classified

## Canonical Binary

**`apps/sexdrive/src/main.rs`** — built by `sexos_build_spec.toml`, staged to
`iso_root/apps/sexdrive`, spawned at runtime as PD 2.

`servers/sexdrive/src/driver.rs` is **dead/abandoned**:
- uses `libsys` crate (does not exist in this project)
- `#![no_std]` attribute placed after `extern crate` (syntax error)
- duplicate `#[panic_handler]` (lines 44 and 49)
- references `park_on_ring`, `sys_wait_vblank_primitive`, `schedule_dag`,
  `execute_dag`, `commit_to_ucgm` — none defined anywhere
- Not listed in any build spec. Not built. Not spawned. Ignore.

## Backend Status: A — Absent

No NVMe or AHCI code exists anywhere in `apps/sexdrive/src/main.rs`.

### What exists in apps/sexdrive

| Symbol | Lines | Purpose |
|--------|-------|---------|
| `xhci_probe_mmio()` | 61–93 | xHCI MMIO probe via `MAP_PCI_BAR(SLOT_USB_HOST, BAR0, 0x1000)` |
| `pdx_try_listen_raw(0)` | 132 | Non-blocking poll for typed block commands |
| `BLOCK_READ/WRITE/SYNC` dispatch | 144–176 | Returns `ERR_NO_DEVICE` for all valid commands |
| `BLOCK_ERR_BAD_CMD/BAD_LEN` | 147–176 | Correct ABI validation |
| `pdx_reply(caller, reply_val)` | 183 | Sends honest reply back to sexfiles |

`xhci_probe_mmio()` uses `SLOT_USB_HOST` for the xHCI USB controller BAR —
completely unrelated to block device I/O.

### What does NOT exist

- No NVMe struct, queue, doorbell, completion ring, MSI-X handler
- No AHCI struct, port list, FIS, command list
- No block device PCI BAR lease or cap grant
- No DMA buffer for block data transfer
- No device enumeration or PCI config space read for block controllers
- No `SLOT_NVME` or `SLOT_STORAGE_HOST` capability slot

## First Dead Hop

`apps/sexdrive/src/main.rs:162–167`:

```rust
} else {
    // Valid command, no real device backend
    serial_println!(
        "[sexdrive.block.typed] cmd={} ERR_NO_DEVICE honest=no_nvme_ahci_backend",
        cmd
    );
    BLOCK_ERR_NO_DEVICE
}
```

This is where a real NVMe submission would go. Replace with actual device I/O.

## What Is Already Wired (Re-use for Real Backend)

| Component | Status | Notes |
|-----------|--------|-------|
| `SLOT_BLOCK = 15` | ✓ Defined in sex-pdx | sexfiles→sexdrive route |
| Typed block ABI (BLOCK_READ/WRITE/SYNC + error codes) | ✓ Defined | No ABI change needed |
| `MAP_PCI_BAR` syscall 43 | ✓ Exists | Used by xhci_probe_mmio; reusable for NVMe BAR |
| sexfiles→sexdrive SLOT_BLOCK cap grant | ✓ In kernel init | Line 249 of init.rs |
| `BLOCK_MAX_XFER = 4096` (one page) | ✓ Defined | Matches NVMe PRPs naturally |

## Prerequisite: Kernel Cap Grant for NVMe BAR

To add real NVMe I/O, sexdrive needs a PCI BAR lease for the NVMe controller.
This requires:
1. A new capability slot (e.g., `SLOT_NVME_HOST`) or repurpose of an unused slot
2. Kernel init: detect NVMe PCI device (class 0x01, subclass 0x08), create BAR lease
3. Grant the lease cap to sexdrive in `kernel/src/init.rs`

**STOP FIRST applies** — this requires kernel/init change. The next prompt must
include kernel scope explicitly.

## Existing xHCI Pattern (Reference for NVMe BAR Map)

```rust
// syscall 43 = MAP_PCI_BAR(cap_slot, bar_index, map_size)
core::arch::asm!(
    "syscall",
    in("rax") 43u64,
    in("rdi") SLOT_USB_HOST,   // cap slot with BAR lease
    in("rsi") 0u64,            // BAR0
    in("rdx") 0x1000u64,       // map size
    lateout("rax") map_va,
    ...
);
// NVMe would use same pattern with a SLOT_NVME_HOST cap and larger map (min 0x1000)
```

## Audit Markers (no build required — documentation only)

No new markers added. The existing log already contains the honest status:

```
[sexdrive.block.typed] cmd=1 ERR_NO_DEVICE honest=no_nvme_ahci_backend
```

This is the ground truth: typed ABI wired, zero backend.

## Next Prompt: SEXDRIVE_DEVICE_DISCOVERY_PROOF_V1

Scope: 
- kernel/src/init.rs — add NVMe PCI device scan + BAR lease + cap grant to sexdrive
- apps/sexdrive/src/main.rs — call MAP_PCI_BAR with new cap, read NVMe BAR0 CAP register
- Add proof marker `[sexdrive.nvme.probe.ok]` or `[sexdrive.nvme.probe.no_device]`
- If QEMU has NVMe device (-drive if=none,id=nvm -device nvme,...), prove BAR maps
- If no QEMU NVMe device, prove graceful no-device fallback

STOP FIRST applies at kernel/init scope. Includes ABI version hash update if any
new sex-pdx constant added.

## Backend Milestone Map

| Milestone | Status |
|-----------|--------|
| Typed block ABI wired (SLOT_BLOCK) | DONE |
| sexfiles sends/receives typed block IPC | DONE |
| sexdrive decodes commands, replies honestly | DONE |
| NVMe BAR cap grant in kernel | NOT STARTED |
| NVMe BAR0 map + CAP register read | NOT STARTED |
| NVMe Admin Queue init (identify) | NOT STARTED |
| NVMe I/O Queue init | NOT STARTED |
| NVMe Read sector → buffer_cap DMA | NOT STARTED |
| sexdrive returns BLOCK_OK + real data | NOT STARTED |
