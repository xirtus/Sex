# SEXUSB_SECOND_SLOT_ENABLE_V1

## Status: IMPLEMENTED

## Summary

After the first HID device reaches SET_CONFIGURATION (and SET_IDLE
completes), the driver now checks `target_port_count > 1`.  If a second
port was collected during the initial scan, an XHCI slot is enabled and
the device is addressed.  No endpoint configuration, HID descriptor
fetch, or interrupt polling happens yet.

---

## Second Port Used

`target_ports[1]` — the second connected port discovered during the
initial PORTSC scan.  With the standard QEMU config (usb-kbd on port 5,
usb-tablet on port 6), this is typically port 6.

---

## Second Slot ID

The XHCI controller assigns the next available slot ID.  Since the first
device typically gets slot 1, the second device gets slot 2
(`s2_slot_id` is read from the Enable Slot completion event).

---

## Context/Ring Allocation

Each resource is independently allocated via `sys_alloc_phys`:

| Resource | Variable | Purpose |
|----------|----------|---------|
| Input context page | `s2_input_phys/va` | ICC + Slot + EP0 context (Address Device input) |
| Device context page | `s2_device_phys/va` | Output device context (written by xHC, linked via DCBAA) |
| EP0 transfer ring | `s2_ep0_ring_phys/va` | EP0 control transfer ring for second device |

**No collision** with the first device's resources because:
- DCBAA entries are indexed by slot ID (slot 1 → offset 8, slot 2 → offset 16)
- `sys_alloc_phys` returns distinct physical pages
- EP0 ring is a separate page from the first device's EP0 ring

---

## Code Insertion Point

Between SET_IDLE completion (line 2592) and Configure Endpoint start
(line 2594).  The second device slot enable is guarded by
`if target_port_count > 1 { ... }`.

---

## Markers

| Marker | Purpose |
|--------|---------|
| `[sexusb.slot2.enable.start] port=N` | Beginning of Enable Slot for second port |
| `[sexusb.slot2.enable.ok] slot=N` | Slot enabled successfully, slot ID returned |
| `[sexusb.slot2.enable.bad]` | Slot enable failed (halt) |
| `[sexusb.slot2.address.start] port=N` | Beginning of Address Device for second port |
| `[sexusb.slot2.address.ok] slot=N port=N` | Device addressed successfully |
| `[sexusb.slot2.address.bad]` | Address failed (halt) |
| `[sexusb.slot2.alloc.bad]` | Page allocation failure (halt) |
| `[sexusb.slot2.map.bad]` | Page mapping failure (halt) |
| `[sexusb.slot2.align.bad]` | Page alignment failure (halt) |
| `[sexusb.slot2.speed.bad]` | Unknown port speed (halt) |

---

## Runtime Proof (expected)

With both QEMU devices connected:

```
[sexusb.ports.collect] count=2 first=5
...
[sexusb.xhci.set_config.complete.ok]
[sexusb.xhci.hid.set_idle.timeout.bad]   (not reached — SET_IDLE succeeds)
[sexusb.slot2.enable.start] port=6
[sexusb.slot2.enable.ok] slot=2
[sexusb.slot2.address.start] port=6
[sexusb.slot2.address.ok] slot=2 port=6
...
[sexusb.hid.keyboard.continuous.start]   (first device poll loop)
```

---

## First-Device Behavior Preserved

The entire second-slot block is inside `if target_port_count > 1 { ... }`.
When only one device is connected (port_count == 1), the block is
elided at compile time due to the constant-branch condition, and the
code path is identical to before this change.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | Added second slot enable + address after SET_IDLE |
| `docs/handoff/SEXUSB_SECOND_SLOT_ENABLE_V1.md` | Created |

---

## Build

```
./scripts/entrypoint_build.sh: PASS
```

---

## Next Phase Prompt

```
MISSION: SEXUSB_SECOND_DEVICE_GET_DESCRIPTOR_V1
Goal: Fetch device descriptor and configuration descriptor for the
second device (target_ports[1], slot 2).  Do not bind HID role yet.

Acceptance:
  - GET_DESCRIPTOR(DEVICE, 8) for second device using its EP0 ring
  - Evaluate Context to update EP0 MPS if different from boot guess
  - GET_DESCRIPTOR(DEVICE, 18) for full device descriptor
  - GET_DESCRIPTOR(CONFIGURATION) header + full config descriptor
  - Log [sexusb.slot2.desc8], [sexusb.slot2.full18], [sexusb.slot2.config]
  - Do NOT parse HID interfaces yet
  - Do NOT configure endpoints or poll
  - Build passes
```
