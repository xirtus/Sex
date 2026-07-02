# SEXNET_NIC_BAR_MAP_PROOF_V1

Date: 2026-05-18

## Scope
Read-only NIC BAR map proof only:
- grant NIC PCI capability to sexnet
- allow MAP_PCI_BAR for Ethernet class/subclass
- map BAR0 in sexnet and read RAL0/RAH0

No ring init, no NIC register writes, no descriptor work, no IRQ changes.

## Files changed
- crates/sex-pdx/src/lib.rs
- sexos_build_spec.toml
- kernel/src/devmgr.rs
- kernel/src/init.rs
- kernel/src/syscalls/mod.rs
- servers/sexnet/src/main.rs

## ABI hash method
Computed using repo gate formula from `scripts/entrypoint_build.sh`:

```bash
{ sha256sum kernel/src/syscalls/mod.rs; sha256sum crates/sex-pdx/src/lib.rs; } | sha256sum | awk '{print $1}'
```

Result:
- `a8545feed4f4a7474be5f631da4118d93c6ef893d33eaa6b2850bc536fe92623`

## Proof command
```bash
./scripts/run_daily_driver_proof.sh /tmp/sexnet_nic_bar_map.log
```

## Marker evidence
- `[kernel.pci.nic.found] 00:03.0 vendor=8086 device=100e`
- `[kernel.cap.nic.grant] pd=13 slot=19`
- `[sexnet.nic.bar.map] va=0x40000034c000 ok=1`
- `[sexnet.nic.mac.read] ral=0x12005452 rah=0x80005634 ok=1`
- `FINAL: PASS (249 gates proved, 14 skipped, 0 faults)`

## Outcome
PASS. Mission constraints preserved (read-only map + volatile reads).
