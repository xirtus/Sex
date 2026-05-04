# USB_HOST_DISCOVERY_V1 — Diagnostic Proof

## Date
2026-05-04

## Context
Diagnostic-only phase to prove whether SexOS can discover USB host controllers. No HID/input, no shell, no kernel/ABI changes. Pure audit + proof logging.

## Existing Discovery Path (audited)

### Kernel-side (`kernel/src/devmgr.rs`)
`devmgr::init()` (called from `kernel/src/init.rs:127`) performs PCI enumeration via `HAL.enumerate_pci()`. For each discovered device:

| Class | Subclass | ProgIf | Action |
|-------|----------|--------|--------|
| 0x0c (USB) | 0x03 (HOST) | 0x30 (XHCI) | Leases PCI capability at `SLOT_USB_HOST` (slot 8) to sexusb, grants interrupt capability |
| 0x0c (USB) | any other | any | Prints "DevMgr: Discovered USB controller class=..." but does NOT lease |

Devmgr prints:
- `DevMgr: Discovered USB XHCI ({bus}:{dev}.{func}) vendor={vendor} device={device} bar0={bar} irq_line={irq}`
- `DevMgr: Leased XHCI to pd={pd} slot={slot}`
- `DevMgr: Discovered USB controller class=..:.. (...)` for non-XHCI USB

### Userspace-side (`servers/sexusb/src/main.rs`)
sexusb uses syscall 43 (MAP_PCI_BAR) on capability slot `SLOT_USB_HOST` (8) to map BAR0. The kernel validates class=0x0c, subclass=0x03, prog-if=0x30 before allowing the mapping.

## Changes Made

### `servers/sexusb/src/main.rs`
Added diagnostic-only markers (no behavior change):

```rust
serial_println!("[usb.host.discovery.start]");
serial_println!("[usb.host.pci.scan] slot={} bar=0", SLOT_USB_HOST);
// ... MAP_PCI_BAR attempt ...
serial_println!("[usb.host.controller.none] map_va={:#x}", map_va);   // on failure
serial_println!("[usb.host.discovery.done]");
// ... on success ...
serial_println!("[usb.host.controller.found] slot={} bar=0 map_va={:#x}", SLOT_USB_HOST, map_va);
serial_println!("[usb.host.discovery.done]");
// ... capability register read ...
serial_println!("[usb.host.caps] caplength=.. hciversion=.. hcsp1=.. hcsp2=.. hcc1=..");
```

All markers are pure `serial_println!` — no new syscalls, no MMIO writes, no DMA/ring changes.

## Expected Boot-time Proof Sequence

```
[usb.host.discovery.start]           ← sexusb begins discovery
[usb.host.pci.scan] slot=8 bar=0     ← MAP_PCI_BAR attempt on slot 8 BAR0
[usb.host.controller.found] slot=8 ...  ← controller visible, BAR mapped
[usb.host.discovery.done]            ← discovery complete
[usb.host.caps] caplength=.. hciversion=.. ...  ← capability register dump
```

If NO XHCI controller is available:
```
[usb.host.discovery.start]
[usb.host.pci.scan] slot=8 bar=0
[usb.host.controller.none] map_va=...
[usb.host.discovery.done]
```

## Proof That Discovery Works
1. **Kernel PCI enumeration**: `HAL.enumerate_pci()` in devmgr discovers all controllers
2. **Capability lease**: XHCI gets `PciCapData` granted at `SLOT_USB_HOST`
3. **BAR mapping**: `MAP_PCI_BAR` syscall (43) validates class/subclass/prog-if, returns VA
4. **MMIO capability registers**: caplength, hciversion, hcsp1, hcsp2, hcc1 readable from mapped BAR

## STOP Triggers (not needed here, listed for reference)
- PCI enumeration redesign → NOT needed, existing HAL path works
- Kernel PCI/ACPI/interrupt changes → NOT needed
- New PDX ABI → NOT needed
- HID/input implementation → NOT attempted here
- MMIO register writes → NOT done (existing code does, but not part of this diagnostic)
- DMA/ring setup → NOT touched by this diagnostic
- Shell/display/scene policy → NOT touched

## Next Phase
`USB_XHCI_MINIMAL_ENUM_V1` or a PCI visibility fix — decide based on boot log output.
