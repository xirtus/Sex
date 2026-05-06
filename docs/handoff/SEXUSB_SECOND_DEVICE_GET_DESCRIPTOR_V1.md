# SEXUSB_SECOND_DEVICE_GET_DESCRIPTOR_V1

## Status: IMPLEMENTED

## Summary

Fetch and log device/configuration descriptors for second XHCI device on
`target_ports[1]` (slot 2). No HID role binding, no endpoint configuration,
no polling. Pure descriptor discovery phase after slot enable and address
device.

## Insertion Point

Inside the existing `if target_port_count > 1 { ... }` block, immediately
after the `[sexusb.slot2.address.ok]` marker (line 2758). The inserted
code runs 4 sequential GET_DESCRIPTOR transfers on slot 2's EP0 ring,
then falls through to the closing brace. The single-device Configure
Endpoint + poll loop after the block is unchanged — it still only
operates on the first device.

## Descriptor Transfers

Each transfer follows the same pattern:
1. Read EP0 dequeue pointer from output device context
   (`s2_device_va + ctx_stride`, EP0 context DW2/DW3)
2. Validate dequeue is within `s2_ep0_ring_phys` range, 16-byte aligned,
   DCS=1, and 4 TRB slots fit before ring end
3. Zero descriptor data buffer (shared `desc_data_va/phys`, PAGE_SIZE)
4. Write 3-TRB chain: Setup Stage (type=2) + Data Stage (type=3, CH=1) +
   Status Stage (type=4, IOC=1) + cycle-stop marker (opposite cycle)
5. Doorbell EP0 on `s2_slot_id` (`mmio_write32(db_base, s2_slot_id * 4, 1)`)
6. Poll shared event ring for Transfer Event (type=32), validate
   `cc==SUCCESS && slot==s2_slot_id && ep==1`

### TD1: GET_DESCRIPTOR(DEVICE, 8) — bMaxPacketSize0

| Field | Value |
|-------|-------|
| Setup d0 | `0x0100_0680` |
| Setup d1 | `0x0008_0000` |
| wLength | 8 |
| Buffer offset | read byte 7 → `s2_mps0` |

Marker: `[sexusb.slot2.desc8.ok] mps=N`

### TD2: GET_DESCRIPTOR(DEVICE, 18) — Full Device Descriptor

| Field | Value |
|-------|-------|
| Setup d0 | `0x0100_0680` |
| Setup d1 | `0x0012_0000` |
| wLength | 18 |
| Read | class (byte 4), subclass (5), protocol (6), vid (8-9), pid (10-11) |

Marker: `[sexusb.slot2.desc.device] class=N subclass=N proto=N vid=N pid=N`

### TD3: GET_DESCRIPTOR(CONFIG, 9) — Config Descriptor Header

| Field | Value |
|-------|-------|
| Setup d0 | `0x0200_0680` |
| Setup d1 | `0x0009_0000` |
| wLength | 9 |
| Read | wTotalLength (bytes 2-3), bNumInterfaces (byte 4) |

Marker: `[sexusb.slot2.cfg9.ok] wTotalLength=N num_interfaces=N`

Validates `wTotalLength >= 9`.

### TD4: GET_DESCRIPTOR(CONFIG, wTotalLength) — Full Config Descriptor

| Field | Value |
|-------|-------|
| Setup d0 | `0x0200_0680` |
| Setup d1 | `(wTotalLength << 16)` |
| wLength | wTotalLength |
| Residue | checked, partial OK |

After TD4, walks the descriptor buffer looking for INTERFACE descriptors
(type=4). Logs each with `[sexusb.slot2.desc.iface]` marker showing
interface number, class, subclass, protocol.

Final marker: `[sexusb.slot2.desc.complete] slot=N port=N`

Note: HID detection is not performed — the interface descriptors are
logged for diagnostics only. No `SingleHidBind` entry is created for the
second device.

## Shared Resource Invariants

| Resource | Shared? | Notes |
|----------|---------|-------|
| `desc_data_va/phys` | ✅ Yes | Zeroed before each TD; first device's descriptor data may be overwritten (first device is past descriptor phase, in Configure Endpoint) |
| `ev_idx` / `ev_dcs` | ✅ Yes | Shared event ring; both devices produce events here. All transfer events are consumed — we validate slot==s2_slot_id for our events |
| `event_ring_phys/va` | ✅ Yes | Same single event ring |
| `db_base` | ✅ Yes | Same doorbell register set; doorbell for slot 2 uses offset `s2_slot_id * 4` |
| `op_base`, `intr_base` | ✅ Yes | Same MMIO base addresses |

| Resource | Independent? | Notes |
|----------|--------------|-------|
| `s2_ep0_ring_va/phys` | ✅ Yes | Separate 4KB ring, not shared with first device's `ep0_ring_va` |
| `s2_cycle` | ✅ Yes | TR cycle bit starts at 1 for second device's ring, tracked independently |
| `s2_ep0_idx` | ✅ Yes | Producer index for second device's ring |
| `s2_device_va/phys` | ✅ Yes | Second device's output device context |
| `s2_input_va/phys` | ✅ Yes | Second device's input context (not used after Address Device) |
| `s2_slot_id` | ✅ Yes | Second device's XHCI slot ID |

## Constants Used

All constants are the same as the first device path:
- `TRB_TYPE_SETUP_STAGE = 2`
- `TRB_TYPE_DATA_STAGE = 3`
- `TRB_TYPE_STATUS_STAGE = 4`
- `TRB_TYPE_TRANSFER_EVENT = 32`
- `TRB_CC_SUCCESS = 1`
- `POLL_BUDGET = 100_000`
- `PAGE_SIZE = 4096`
- `TRB_SIZE = 16`

## Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | Add 4 descriptor TDs inside `if target_port_count > 1` block after address.ok |
| `docs/handoff/SEXUSB_SECOND_DEVICE_GET_DESCRIPTOR_V1.md` | Created |

## Build

`./scripts/entrypoint_build.sh` PASS.

## Markers

| Marker | Meaning |
|--------|---------|
| `[sexusb.slot2.desc8.ok] mps=N` | TD1 complete, bMaxPacketSize0 = N |
| `[sexusb.slot2.desc8.bad]` | TD1 failed (timeout or bad event) |
| `[sexusb.slot2.desc.device] class=N subclass=N proto=N vid=N pid=N` | TD2 complete, device descriptor parsed |
| `[sexusb.slot2.full18.bad]` | TD2 failed |
| `[sexusb.slot2.cfg9.ok] wTotalLength=N num_interfaces=N` | TD3 complete, config header parsed |
| `[sexusb.slot2.cfg9.bad]` | TD3 failed |
| `[sexusb.slot2.cfg_full.ok] received_len=N` | TD4 complete, full config received |
| `[sexusb.slot2.cfg_full.bad]` | TD4 failed |
| `[sexusb.slot2.desc.iface] idx=N if=N class=N subclass=N proto=N` | INTERFACE descriptor found during walk |
| `[sexusb.slot2.desc.complete] slot=N port=N` | All descriptor fetches done |

## Next Phase

**SEXUSB_SECOND_DEVICE_HID_BIND_V1**: After descriptors are fetched and
logged, parse the interface descriptors to detect HID boot keyboard/mouse/
tablet, create a `SingleHidBind` (or `HidDevice` if the struct has been
refactored) for the second device, then configure its interrupt endpoint
and start polling.
