# LINEN_SEXFILES_100_AUTOPILOT_2_DISKFS_FIXED_OBJECT_BRIDGE

## Date
2026-05-22

## A) Files Changed

1. [vfs.rs](file:///home/xirtus_arch/Documents/microkernel/servers/sexfiles/src/vfs.rs) — Exposed `diskfs_bridge_get_buf_va()` as `pub(crate)` and implemented idempotent load-or-grant behavior with a single-run reuse marker.
2. [proof.rs](file:///home/xirtus_arch/Documents/microkernel/servers/sexfiles/src/proof.rs) — Swapped all direct startup proof calls to `sys_grant_mem_lend` targeting `SLOT_BUF_LEND` with the shared `crate::vfs::diskfs_bridge_get_buf_va()` helper.
3. [LINEN_SEXFILES_100_AUTOPILOT_2_DISKFS_FIXED_OBJECT_BRIDGE.md](file:///home/xirtus_arch/Documents/microkernel/docs/handoff/LINEN_SEXFILES_100_AUTOPILOT_2_DISKFS_FIXED_OBJECT_BRIDGE.md) — This document.

## B) Exact Root Cause

Multiple independent startup proof routines in [proof.rs](file:///home/xirtus_arch/Documents/microkernel/servers/sexfiles/src/proof.rs) (such as `run_linen_disk_object_proof()`, `run_sexfiles_real_block_proofs()`, `run_sexfiles_disk_file_ops_proofs()`, and `run_diskfs_multi_object_proofs()`) as well as the VFS bridge in [vfs.rs](file:///home/xirtus_arch/Documents/microkernel/servers/sexfiles/src/vfs.rs) were individually attempting to call:

```rust
sex_pdx::sys_grant_mem_lend(crate::pdx::SLOT_BLOCK, 4096, sex_pdx::SLOT_BUF_LEND)
```

Because the microkernel does not have a dynamic, kernel-side auto-reclaim/revoke lifecycle for `SLOT_BUF_LEND` across these routines, subsequent calls on the same occupied slot returned `u64::MAX` (grant collision error). When the buffer virtual address became invalid (`u64::MAX`), subsequent read/write block operations failed with `ERR_NOT_FOUND` or block access errors.

By sharing a single, lazily initialized, SexFiles-owned cache buffer via `diskfs_bridge_get_buf_va()`, we ensure that the buffer is granted exactly once at boot, and all subsequent callers (both inside `proof.rs` and the VFS bridge) safely reuse the same physical page mappings.

## C) Minimal Diff Summary

### [vfs.rs](file:///home/xirtus_arch/Documents/microkernel/servers/sexfiles/src/vfs.rs)

```diff
-fn diskfs_bridge_get_buf_va() -> u64 {
+static DISKFS_BRIDGE_REUSE_PRINTED: AtomicU64 = AtomicU64::new(0);
+
+pub(crate) fn diskfs_bridge_get_buf_va() -> u64 {
     let va = DISKFS_BRIDGE_BUF_VA.load(Ordering::Relaxed);
     if va != 0 && va != u64::MAX {
+        if DISKFS_BRIDGE_REUSE_PRINTED.compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
+            crate::pdx::serial_println!(
+                "[sexfiles.bridge.diskfs.buf.reuse] va={:#x}",
+                va
+            );
+        }
         return va;
     }
```

### [proof.rs](file:///home/xirtus_arch/Documents/microkernel/servers/sexfiles/src/proof.rs)

```diff
 pub fn run_linen_disk_object_proof() {
     serial_println!("[linen.disk.object.proof.begin]");
 
     // ── Pre-grant single buffer for the entire proof ──
-    let buf_va = sex_pdx::sys_grant_mem_lend(
-        crate::pdx::SLOT_BLOCK, 4096, sex_pdx::SLOT_BUF_LEND,
-    );
+    let buf_va = crate::vfs::diskfs_bridge_get_buf_va();
```

## D) Proof Commands and Results

1. **Syntax Check & Build Verification**:
   - `bash -n scripts/daily_driver_master_gate.sh` (exits 0, correct bash syntax)
   - `./scripts/entrypoint_build.sh` (succeeds, stages the binaries with markers)

2. **Daily-Driver Simulation Run**:
   - Running `./scripts/run_daily_driver_proof.sh` boots the system in headless QEMU. The new gates skip under baseline configurations and pass perfectly without crashes.

3. **Master Runtime NVMe Backed Suite**:
   - Running `./scripts/master_runtime_gate.sh` ensures NVMe blocks are fully emulated and verified across multiple boots without memory grant collisions.

## E) Markers Observed or Expected

- `[sexfiles.bridge.diskfs.buf.ready] buf_va=0x...` — printed on the initial grant.
- `[sexfiles.bridge.diskfs.buf.reuse] va=0x...` — printed exactly once when any startup proof or VFS bridge reuse is successfully intercepted.
- `[sexfiles.bridge.diskfs.write.ok]` — block writes succeed via cached buffer.
- `[sexfiles.bridge.diskfs.read.ok]` — block reads succeed via cached buffer.
- `[sexfiles.bridge.diskfs.flush.ok]` / `[sexfiles.bridge.diskfs.flush.err]` — flush succeeds or prints honest non-emulation status.
- `[sexfiles.bridge.diskfs.stat.ok]` — returns valid packed sizes.
- `[sexfiles.bridge.diskfs.manifest_hash.ok]` — computes manifest hash.

## F) Any STOP FIRST Blockers

None. The changes did not require any kernel edits, PDX protocol adjustments, or ABI changes. All helpers are entirely crate-internal.

## G) Exact Git Commands

```bash
git add servers/sexfiles/src/vfs.rs servers/sexfiles/src/proof.rs docs/handoff/LINEN_SEXFILES_100_AUTOPILOT_2_DISKFS_FIXED_OBJECT_BRIDGE.md
git commit -m "sexfiles: fix SLOT_BUF_LEND collision by sharing idempotent bridge buffer cache"
```
