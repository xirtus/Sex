# SEXDRIVE_STORAGE_100_AP1_NO_IOQ_READY_AUDIT

## Date
2026-05-22

## 1. Summary
- **AP1 Audit Only**: Conducted a comprehensive diagnostic audit of the NVMe block storage readiness failures.
- **No Durable Payload Success Claimed**: Confirmed that real sector-level write/read operations are blocked and marked honestly as `SKIP` at the master gate level. No fake success is introduced.

---

## 2. Runtime Evidence
The following markers from the latest proof log (`/tmp/sexos_daily_driver_proof.log`) capture the failure sequence:

1. **Hardware discovery (absent NVMe)**:
   ```log
   1395:[kernel.pci.nvme.absent]
   ```
2. **PCI BAR resolution fails**:
   ```log
   1526:[sexdrive.nvme.bar.resolve.begin] slot=16 bar=0 [sexdrive.nvme.bar.resolve.err] map_va=0xffffffffffffffff [sexdrive.device.no_nvme_cap]
   ```
3. **SexFiles calls SexDrive for BLOCK_READ and receives `no_ioq_ready` (`status=4`)**:
   ```log
   5145:[sexfiles.diskfs.typed.call] cmd=BLOCK_READ offset=0xffc00 size=512 buf_cap=0x11
   5146:[sexfiles.diskfs.call] slot=15 opcode=0x1 arg0=0xffc00 arg1=0x200 arg2=0x11
   5532:[sexdrive.block.typed.recv] cmd=1 offset=0xffc00 size=512 buf_cap=0x11 caller=11
   5533:[sexdrive.block.read.api.recv] offset=0xffc00 size=512 buf_cap=0x11
   5534:[sexdrive.block.read.handoff.begin] offset=0xffc00 size=512 buf_cap=0x11
   5537:[sexdrive.bufcap.map.ok] fill_va=0x40000034c000
   5538:[sexdrive.block.read.handoff.nvme.begin] offset=0xffc00 size=512 dst_va=0x40000034c000
   5539:[sexdrive.block.read.handoff.err] reason=no_ioq_ready
   5540:[sexblock.abi.reply.encode] caller=11 status=4
   5541:[sexdrive.block.typed.reply] cmd=1 caller=11 status=4
   5626:[sexfiles.diskfs.reply] status=0x0 value=0x4
   5627:[sexfiles.diskfs.typed.reply] cmd=BLOCK_READ status=4
   ```
4. **SexFiles calls SexDrive for BLOCK_WRITE and receives `no_ioq_ready` (`status=4`)**:
   ```log
   5630:[sexfiles.diskfs.typed.call] cmd=BLOCK_WRITE offset=0xffc00 size=512 buf_cap=0x11
   5631:[sexfiles.diskfs.call] slot=15 opcode=0x2 arg0=0xffc00 arg1=0x200 arg2=0x11
   5689:[sexdrive.block.typed.recv] cmd=2 offset=0xffc00 size=512 buf_cap=0x11 caller=11
   5690:[sexdrive.block.write.api.recv] offset=0xffc00 size=512 buf_cap=0x11
   5692:[sexdrive.write.guard.begin] offset=0xffc00 size=512 buf_cap=0x11 proof_mode=1
   5696:[sexdrive.bufcap.map.ok] fill_va=0x40000034d000
   5697:[sexdrive.nvme.write.err] reason=no_ioq_ready
   5698:[sexblock.abi.reply.encode] caller=11 status=4
   5699:[sexdrive.block.typed.reply] cmd=2 caller=11 status=4
   5781:[sexfiles.diskfs.reply] status=0x0 value=0x4
   5782:[sexfiles.diskfs.typed.reply] cmd=BLOCK_WRITE status=4
   ```

---

## 3. Exact Source Root Cause
- **File**: [apps/sexdrive/src/main.rs](file:///home/xirtus_arch/Documents/microkernel/apps/sexdrive/src/main.rs)
- **Function**: `nvme_probe_bar()`
- **Line 1035 Condition**:
  ```rust
  if map_va == u64::MAX || map_va == 0 {
      serial_println!(
          "[sexdrive.nvme.bar.resolve.begin] slot={} bar={} [sexdrive.nvme.bar.resolve.err] map_va={:#x} [sexdrive.device.no_nvme_cap]",
          SLOT_NVME_HOST, 0u64, map_va
      );
      return;
  }
  ```
  Since the PCI NVMe controller device is absent in QEMU during the daily-driver proof run, `syscall 43` (`MAP_PCI_BAR`) returns `u64::MAX`, causing an early exit from `nvme_probe_bar()`. Because of this, the entire NVMe admin and I/O submission/completion queue setup is bypassed, and the static variable `NVME_IO_STATE.ready` remains `false`.
  
  In the API read/write handlers:
  - **Read (line 319)**:
    ```rust
    if !NVME_IO_STATE.ready {
        serial_println!("[sexdrive.block.read.handoff.err] reason=no_ioq_ready");
        return BLOCK_ERR_NO_DEVICE; // returns status=4
    }
    ```
  - **Write (line 678)**:
    ```rust
    if !NVME_IO_STATE.ready {
        serial_println!("[sexdrive.nvme.write.err] reason=no_ioq_ready");
        return BLOCK_ERR_NO_DEVICE; // returns status=4
    }
    ```

---

## 4. Exact Call Path
1. **SexFiles DiskFS bridge** receives a read/write operation.
2. **SexFiles** emits a typed call to SexDrive via the block slot:
   - `BLOCK_READ`/`BLOCK_WRITE` call targeted to `SLOT_BLOCK` (slot 15).
3. **SexDrive** receives the request in `sexdrive.block.typed.recv` (`cmd=1` / `cmd=2`).
4. **SexDrive** checks I/O queue readiness state:
   - Evaluates `NVME_IO_STATE.ready` (which is `false` due to the early-return during discovery).
5. **SexDrive** prints `reason=no_ioq_ready` and returns `BLOCK_ERR_NO_DEVICE` (`status=4`).
6. **SexFiles** receives `status=4` reply and handles it gracefully as an honest skipped storage blocker.

---

## 5. Root Cause Classification
- **CASE 1 — Device absent**: The QEMU configuration in the daily-driver proof runner script (`scripts/run_daily_driver_proof.sh`) lacks NVMe arguments.
- **CASE 8 — Gate/profile mismatch**: The proof orchestrator does not enable NVMe backing-hardware parameters for this specific target run, causing the device discovery process to report `[kernel.pci.nvme.absent]`.

---

## 6. What is Already Proven
- **Bridge Reachability**: Linen -> SexFiles VFS -> DiskFS bridge path operates correctly and reaches the SexDrive block device boundary.
- **IPC Mechanics**: Inter-domain typed call and reply encoding/decoding are robust; status code `status=4` is successfully returned without domain faults.
- **Zero Faults**: System executes without kernel page faults (`#PF`), General Protection Faults (`#GP`), or scheduler hangs.

---

## 7. What is Not Proven
- **I/O Queue Initialization**: Real administrative/IO submission & completion queues are not configured or proven online.
- **Durable Media Write/Read**: True sector data operations targeting volatile or persistent NVMe physical sectors are not yet completed.
- **Durable Payload Verification**: Genuinely writing, reading, and matching the 128-byte payload is blocked by lack of hardware backing.

---

## 8. AP2 Recommended Mission
- **Mission Name**: `SEXDRIVE_STORAGE_100_AP2_IOQ_READY`
- **Allowed Target Files**:
  - `scripts/run_daily_driver_proof.sh` (to configure QEMU to launch with NVMe drive arguments when the storage proof environment is requested)
  - `scripts/daily_driver_master_gate.sh` (to evaluate real write/read passes)
  - `apps/sexdrive/src/main.rs` (for any real hardware timing or setup adjustments)
- **STOP FIRST Boundaries**:
  - NO kernel edits (unless a critical PCI capability grant/IRQ bug is found, which must stop for approval first)
  - NO `sex-pdx` ABI changes
  - NO Linen edits
  - NO faking storage success; the bytes must genuinely read and write to the QEMU back-end drive.

---

## 9. Proof Gates AP2 Must Satisfy
- **SexDrive IOQ Ready Marker**: `[sexdrive.nvme.ioq.ready] qid=1 depth=16` online.
- **Single Block Write/Read Proof**: Successful sector-level MMIO interactions.
- **SexFiles DiskFS bridge write/read OK**: Returns `status=0` instead of `status=4`.
- **Durable Payload Match**: The 128-byte write/read roundtrip successfully completes with a real content match.

---

## 10. Exact Commands Run
```bash
grep -n -E "sexdrive|sexblock|nvme|ioq|ioq_ready|no_ioq_ready|queue|SQ|CQ|doorbell|BAR|MSI|MSI-X|status=4|BLOCK_READ|BLOCK_WRITE|SLOT_BLOCK|sexfiles\.diskfs|sexfiles\.bridge\.diskfs" /tmp/sexos_daily_driver_proof.log | tail -400 || true
bash -n scripts/daily_driver_master_gate.sh
```
