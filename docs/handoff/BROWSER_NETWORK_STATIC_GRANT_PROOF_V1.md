# BROWSER_NETWORK_STATIC_GRANT_PROOF_V1

## Mission Result
**PASS**: Kaleidoscope successfully received the `SLOT_NET` capability via static boot-time grant and performed a scalar IPC call to `sexnet`.

## Exact Diff Summary

### 1. `kernel/src/init.rs`
- Added `kaleidoscope` to the `module_paths` list to enable boot-time spawning.
- **Critical PKU Fix**: Inserted `RESERVED_SHARED` at index 13 to force Kaleidoscope to `domain_id` 15. This avoids `domain_id` 14, which is hardcoded in `capability.rs` for shared memory assertions, preventing Kaleidoscope from accidentally self-isolating its own stack/memory.
- Implemented static grant of `SLOT_NET` (slot 18) to Kaleidoscope pointing to `sexnet` (domain 13).
- Emitted kernel spawn and grant markers.

### 2. `apps/kaleidoscope/src/main.rs`
- Fixed multiple API mismatches (removed `Pdx` import, updated `SilkWindow::create` args, updated `Rect` fields to `width`/`height`).
- Added a scalar `pdx_call(SLOT_NET, 0x200, 0, 0, 0)` at startup in `App::new`.
- Emitted browser-side routing markers.

### 3. `servers/silk-shell/src/main.rs`
- Added the `[collar.net.policy.browser]` future-policy stub marker.

### 4. `limine.cfg`
- Registered the `kaleidoscope` module so the kernel can locate and spawn it.

### 5. `sexos_build_spec.toml`
- Added `apps/kaleidoscope/Cargo.toml` to the allowed crates whitelist.
- Added a `build_kaleidoscope` stage to the deterministic build sequence.

### 6. `Cargo.toml`
- Added `apps/kaleidoscope` to the workspace members list.

### 7. `crates/silk-client/src/lib.rs`
- Added the `commit` method to `SilkWindow` and implemented it via the `OP_WINDOW_SUBMIT` PDX call to `SLOT_DISPLAY`.

## Proof Evidence
- **Proof command**: `./scripts/run_daily_driver_proof.sh /tmp/browser_network_static_grant_proof.log`
- **Log path**: `/tmp/browser_network_static_grant_proof.log`
- **Marker results**:
  - `[kernel.spawn.kaleidoscope] id=14` (Note: Kernel PD index is 14, but Domain ID/PKEY is 15).
  - `[slot.net.grant.browser.ok] slot=18 kaleido_pd=15`
  - `[browser.slot.net.static_grant.begin]`
  - `[browser.slot.net.route.call] status=0` (Successful scalar IPC return from SexNet).
  - `[browser.slot.net.static_grant.proof.done] ok=1 network=0`

The system correctly enforced PKEY 15 for the browser and correctly routed the capability call to the network server.

Next Mission: **SEXNET_PACKED_TEXT_REPLY_PLAN_V1**
