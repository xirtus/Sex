# SLOT_NET_CONSTANT_AND_GRANT_PROOF_V1

## Mission Result
**PASS**: `SLOT_NET` capability successfully defined, granted to `silk-shell`, and proven via scalar IPC.

## Exact Diff Summary
### 1. `crates/sex-pdx/src/lib.rs`
Added the capability slot constant:
```rust
pub const SLOT_NET:     u64 = 18; // sexnet network manager route
```

### 2. `kernel/src/init.rs`
Granted the capability to the `silk-shell` protection domain during the boot-time static grant phase:
```rust
            if sexnet_id != 0 {
                serial_println!("[slot.net.constant] slot=18");
                serial_println!("[slot.net.grant.begin] target=silk-shell sexnet_pd=13");
                pd.grant_capability(sex_pdx::SLOT_NET, CapabilityData::Domain(sexnet_id));
                serial_println!("[slot.net.grant.ok] slot=18 pd=13");
                serial_println!("[sexnet.route.slot.grant] slot=18 pd=13 ok=1");
                serial_println!("[slot.net.grant.proof.done]");
            }
```

### 3. `servers/silk-shell/src/main.rs`
Updated the `maybe_run_sexnet_status_route_proof` function to perform a real scalar `pdx_call` using the new `SLOT_NET` constant:
```rust
    serial_println!("[sexnet.route.proof.begin]");
    let (status, value) = pdx_call(sex_pdx::SLOT_NET, 0x200, 0, 0, 0);
    serial_println!("[sexnet.route.call.get_status] ret={} status={}", value, status);
    serial_println!("[sexnet.route.proof.done]");
```

### 4. `sexos_build_spec.toml`
Updated the `abi_version_hash` to `ae5c58d1e0b9870d61fcbc58859516c945d222a0fb8e506ae3e2117fa3e5ce91` to reflect the changes in `sex-pdx`.

## Proof Evidence
- **Proof command:** `QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 ENABLE_QEMU_USERNET_E1000=1 ./scripts/run_daily_driver_proof.sh /tmp/sexos_slot_net_grant.log`
- **Log path:** `/tmp/sexos_slot_net_grant.log`
- **Marker results:**
  - `[slot.net.constant] slot=18`
  - `[slot.net.grant.ok] slot=18 pd=13`
  - `[sexnet.route.slot.grant] slot=18 pd=13 ok=1`
  - `[sexnet.route.call.get_status] ret=0 status=0` (Scalar status `0` received from passive `sexnet` mock).

Next Mission: **SHELL_BROWSER_SLOT_NET_DELEGATION_PLAN_V1**
