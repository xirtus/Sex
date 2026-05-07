# SEXFILES_SEXDRIVE_ROUTE_PROOF_V1

## 1. Exact Current Route Map
Route: **WIRED — SLOT_BLOCK (15) — sexfiles→sexdrive capability granted.**

### Route Flow
```
sexfiles.diskfs.call  ──(pdx_call SLOT_BLOCK=15)──▶  sexdrive.dma.recv  ──(pdx_reply)──▶  sexfiles.diskfs.reply
```

### Infrastructure
- `crates/sex-pdx/src/lib.rs`: `SLOT_BLOCK = 15` defined in the well-known slot table.
- `kernel/src/init.rs`: `sexfiles_id` granted `CapabilityData::Domain(sexdrive_id)` at `SLOT_BLOCK`.
- `servers/sexfiles/src/backends/diskfs.rs`: `DiskFs::diskfs_block_call()` sends `pdx_call(SLOT_BLOCK, opcode, args)` with `[sexfiles.diskfs.call]` / `[sexfiles.diskfs.reply]` proof markers.
- `apps/sexdrive/src/main.rs`: Non-blocking `pdx_try_listen_raw(0)` poll with `[sexdrive.dma.recv]` / `[sexdrive.dma.reply]` proof markers in the framebuffer loop.

## 2. Root Cause If Broken
Resolved. The previous blockers were:
- **Kernel/ABI Blocker**: ~~No SLOT_BLOCK in sex-pdx~~ → **RESOLVED**: `pub const SLOT_BLOCK: u64 = 15;`
- **Kernel Cap Grant**: ~~No sexfiles→sexdrive capability~~ → **RESOLVED**: `pd.grant_capability(sex_pdx::SLOT_BLOCK, CapabilityData::Domain(sexdrive_id));`
- **Userland Call Path**: ~~No DiskFS call to sexdrive~~ → **WIRED**: `DiskFs::diskfs_block_call()` exists.
- **Userland Listen Path**: ~~No sexdrive DMA listen loop~~ → **MINIMAL**: Non-blocking poll with echo reply in framebuffer loop.

## 3. Files Changed
- `crates/sex-pdx/src/lib.rs` — +1 slot (SLOT_BLOCK = 15)
- `kernel/src/init.rs` — +4 lines grant block
- `servers/sexfiles/src/backends/diskfs.rs` — +2 lines import, +15 lines `diskfs_block_call`
- `servers/sexfiles/src/proof.rs` — updated blocker report → ROUTE_WIRED
- `apps/sexdrive/src/main.rs` — +1 line import, +13 lines DMA listen poll

## 4. Minimal Diff Summary
```diff
# crates/sex-pdx/src/lib.rs
+pub const SLOT_BLOCK: u64 = 15; // sexdrive block/DMA service

# kernel/src/init.rs
+    if sexfiles_id != 0 && sexdrive_id != 0 {
+        pd.grant_capability(sex_pdx::SLOT_BLOCK, CapabilityData::Domain(sexdrive_id));
+        serial_println!("[kernel.cap.block] sexfiles->sexdrive slot={}", sex_pdx::SLOT_BLOCK);
+    }

# servers/sexfiles/src/backends/diskfs.rs
+use sex_pdx::{pdx_call, serial_println, SLOT_BLOCK};
+    pub fn diskfs_block_call(opcode: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
+        serial_println!("[sexfiles.diskfs.call] slot={} ...", SLOT_BLOCK, ...);
+        let (status, value) = pdx_call(SLOT_BLOCK, opcode, arg0, arg1, arg2);
+        serial_println!("[sexfiles.diskfs.reply] status={:#x} value={:#x}", status, value);
+        value
+    }

# apps/sexdrive/src/main.rs
+use sex_pdx::{..., pdx_reply, pdx_try_listen_raw, ...};
+        if let Some(msg) = pdx_try_listen_raw(0) {
+            serial_println!("[sexdrive.dma.recv] type_id={:#x} ...", ...);
+            pdx_reply(msg.caller_pd, msg.arg0);
+            serial_println!("[sexdrive.dma.reply] caller={} value={:#x}", ...);
+        }
```

## 5. Build Command
```bash
./build_payload.sh
# or per-crate:
cargo build --manifest-path servers/sexfiles/Cargo.toml
cargo build --manifest-path apps/sexdrive/Cargo.toml
```

## 6. Runtime Grep Command
```bash
./scripts/master_runtime_gate.sh --probe 25 --keep-log | grep -E 'dma|diskfs|sexdrive|block\.proof|kernel\.cap\.block'
```
Or with SEXOS_SEXFILES_REAL_BLOCK_PROOF enabled:
```bash
SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log | grep -E 'sexfiles\.diskfs\.(call|reply)|sexdrive\.dma\.(recv|reply)|kernel\.cap\.block|block\.proof\.route_demo'
```

## 7. Pass/Fail Proof Markers
- `[sexfiles.diskfs.call]` — **WIRED** in `DiskFs::diskfs_block_call()` (diskfs.rs:228)
- `[sexdrive.dma.recv]` — **WIRED** in framebuffer loop `pdx_try_listen_raw(0)` poll (main.rs:125)
- `[sexdrive.dma.reply]` — **WIRED** echo reply after recv (main.rs:131)
- `[sexfiles.diskfs.reply]` — **WIRED** after `pdx_call` returns (diskfs.rs:233)

## 8. Remaining Blockers
- **DmaCall/DmaReply ABI layout**: Currently uses raw opcode/arg0/arg1/arg2. A typed `DmaCall { block_offset, block_len, buffer }` / `DmaReply { status, data }` layout requires STOP FIRST.
- **sexdrive DMA backend**: The sexdrive echo reply is a proof-of-route marker only. Real block I/O (NVMe/AHCI driver, actual DMA transfers) is pending.
- **DiskFS persistent media**: DiskFS still operates on an in-memory scaffold. Calling `diskfs_block_call` bridges to sexdrive but the data path is not wired to on-disk storage.

## 9. Slot Selection Rationale
**Chosen: SLOT_BLOCK = 15**
- Slots 1-14 are assigned in `sex-pdx`.
- Slot 9 is kernel-local `SLOT_USB_SEXINPUT` (not in sex-pdx public table).
- Slot 15 is the first unused slot, preserving backward compatibility.
- No existing slot values were changed.
