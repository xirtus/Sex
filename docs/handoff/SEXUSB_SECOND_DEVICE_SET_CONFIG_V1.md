# SEXUSB_SECOND_DEVICE_SET_CONFIG_V1

## Status: IMPLEMENTED

## Summary

Issue SET_CONFIGURATION(1) for slot2 usb-tablet after HID role
classification. No interrupt endpoint configured, no polling, no pointer
events. Pure configuration phase after role binding.

## Insertion Point

Inside `if target_port_count > 1 { ... }` block, after
`[sexusb.slot2.desc.complete]` marker and before the closing `}`.
The block now runs: Enable Slot → Address Device → 4x Descriptor TDs →
HID Role Classification → **SET_CONFIGURATION** → (close block).

## SET_CONFIGURATION Transfer

A no-data-stage control transfer (2 TRBs + stop marker):

| USB Field | Value | Meaning |
|-----------|-------|---------|
| bmReqType | 0x00 | Host-to-device, Standard, Device |
| bRequest | 0x09 | SET_CONFIGURATION |
| wValue | 0x0001 | Configuration 1 |
| wIndex | 0x0000 | Zero |
| wLength | 0x0000 | No data stage |

### TRB Chain

1. **Setup Stage** (type=2): `TRT=00` (no data stage),
   `d0 = 0x0001_0900` ((1 << 16) | 0x0900),
   `d1 = 0`, `DW2 = 8`, QEMU bit6 inline marker,
   cycle = `s2_cycle`
2. **Status Stage** (type=4): `DIR=IN` (bit 16=1 for device status),
   `IOC=1` (bit 5), cycle = `s2_cycle`
3. **Cycle-stop**: opposite cycle at index+2

### Dequeue Read and Validation

EP0 dequeue pointer read from output device context:
- `s2_device_va + ctx_stride` (EP0 context, offset DW2/DW3)
- Validated: DCS=1, within `s2_ep0_ring_phys` range, 16-byte aligned,
  `index + 2 < PAGE_SIZE / TRB_SIZE` (need 3 slots: 2 TRBs + stop)

### Event Validation

Transfer Event must have `cc == SUCCESS && slot == s2_slot_id && ep == 1`.
Residue must be 0 (no data stage should have zero residue).

## Markers

| Marker | Condition |
|--------|-----------|
| `[sexusb.slot2.set_config.start] slot=N config=1` | Before TRB write |
| `[sexusb.slot2.set_config.reject] reason=deq_bad ptr=N dcs=N` | Dequeue pointer invalid |
| `[sexusb.slot2.set_config.reject] reason=ring_overflow idx=N` | Not enough ring space |
| `[sexusb.slot2.set_config.event.bad] cc=N slot=N ep=N` | Wrong completion event |
| `[sexusb.slot2.set_config.reject] reason=timeout` | POLL_BUDGET exhausted |
| `[sexusb.slot2.set_config.reject] reason=residue residue=N` | Non-zero residual |
| `[sexusb.slot2.set_config.ok] slot=N` | Configuration confirmed |

## State After SET_CONFIGURATION

After this phase, the slot2 device has:
- Enabled XHCI slot (slot ID 2)
- Assigned USB address
- Device descriptor fetched and logged
- Config descriptor fetched and walked
- HID role classified (PointerTablet)
- Configuration 1 selected (**new in this phase**)

Still NOT done:
- Interrupt endpoint not configured
- No interrupt ring allocated
- No polling
- No HID report fetching
- No pointer events forwarded

## Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | Add SET_CONFIGURATION on slot2 EP0 ring after desc.complete |
| `docs/handoff/SEXUSB_SECOND_DEVICE_SET_CONFIG_V1.md` | Created |

## Build

`./scripts/entrypoint_build.sh` PASS.

## Regression Check

| Check | Status |
|-------|--------|
| First device SET_CONFIGURATION unchanged | ✅ At line 2427, not touched |
| First device poll loop unchanged | ✅ At line 3144+, not touched |
| SingleHidBind unchanged | ✅ Not touched |
| Event ring shared correctly | ✅ Validated by slot ID |
| No endpoint config for slot2 | ✅ Not added |
| No polling for slot2 | ✅ Not added |
| No pointer events | ✅ Not sent |
| No array refactor | ✅ Local variables only |

## Next Phase

**SEXUSB_SECOND_DEVICE_CONFIGURE_ENDPOINT_V1**: After SET_CONFIGURATION,
the device's interrupt endpoint is available. Next steps:

1. Re-read config descriptor (or use cached data) to find interrupt-IN
   endpoint bEndpointAddress, wMaxPacketSize, bInterval
2. Allocate interrupt transfer ring and report buffer for slot2
3. Issue Configure Endpoint command on slot2
4. Start interrupt polling on slot2's endpoint
5. Demux events by slot ID in the poll loop

Prerequisite: the current single-device poll loop at line 3144+ needs
to be aware of a second device's interrupt endpoint. This may require
an event demux or a second poll loop.
