# SEXUSB_SECOND_HID_ROLE_BIND_V1

## Status: IMPLEMENTED

## Summary

Classify and store the second device HID role from slot2 descriptors.
No interrupt endpoint configured, no polling, no pointer events sent.
Pure role classification phase after descriptor fetch.

## Scope

HID role classification for slot2:

- Reuse existing `classify_single_hid_role()` helper
- Detect HID interface (bInterfaceClass == 0x03) during config descriptor walk
- Classify QEMU usb-tablet (class=0x03, subclass=0x00, proto=0x00) as
  `HidRole::PointerTablet`
- Store role + interface number in local variables `s2_hid_role` / `s2_hid_iface`
- Log classification markers
- Do NOT configure endpoint, do NOT poll, do NOT send events

## Changes

### `servers/sexusb/src/main.rs`

**1. `HidRole` enum** (line 191): Added `#[derive(Debug)]` to support
`{:?}` formatting in slot2 role markers. No variants changed.

**2. Slot2 interface walk** (inside `if target_port_count > 1` block,
replaces lines ~3079-3105):

Added HID classification variables:
```rust
let mut s2_found_hid_keyboard: bool = false;
let mut s2_found_hid_tablet: bool = false;
let mut s2_found_hid_mouse: bool = false;
let mut s2_hid_iface: u8 = 0;
```

Inside the `b_type == 4` (INTERFACE) handler, added HID detection:
```rust
let is_hid = b_class == 0x03;
if is_hid {
    let is_boot_mouse = (b_sub == 0x01) && (b_proto == 0x02);
    let is_boot_keyboard = (b_sub == 0x01) && (b_proto == 0x01);
    // ... classify and log
}
```

After the walk, final role determined via `classify_single_hid_role()`:
```rust
let s2_hid_role: HidRole = classify_single_hid_role(
    s2_found_hid_keyboard, s2_found_hid_tablet, s2_found_hid_mouse);
```

### Classification Logic for QEMU usb-tablet

The QEMU usb-tablet presents:
- bInterfaceClass = 0x03 → `is_hid = true`
- bInterfaceSubClass = 0x00 → not boot mouse (needs subclass=1, proto=2)
- bInterfaceProtocol = 0x00 → not boot keyboard (needs subclass=1, proto=1)
- `b_proto != 0x01` is true → falls into `found_hid_tablet = true` branch

This correctly classifies as `HidRole::PointerTablet`.

### Markers Added

| Marker | Meaning |
|--------|---------|
| `[sexusb.slot2.hid.classify] iface=N role=tablet subclass=N proto=N` | Per-interface HID classification during walk |
| `[sexusb.slot2.hid.classify] iface=N role=keyboard` | HID keyboard detected |
| `[sexusb.slot2.hid.classify] iface=N role=mouse` | HID mouse detected |
| `[sexusb.slot2.hid.classify] iface=N role=unknown_hid` | HID but unrecognized |
| `[sexusb.slot2.hid.role] role=PointerTablet iface=N` | Final classified role |
| `[sexusb.slot2.hid.pointer.ready] iface=N` | Slot2 has a pointer device ready |
| `[sexusb.slot2.desc.config] wTotalLength=N interfaces=N hid_role=PointerTablet` | Updated config summary |

## Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | Add Debug derive to HidRole; add HID classification in slot2 walk |
| `docs/handoff/SEXUSB_SECOND_HID_ROLE_BIND_V1.md` | Created |

## Build

`./scripts/entrypoint_build.sh` PASS.

## Runtime Proof (Expected)

With QEMU `usb-kbd` + `usb-tablet`:

```
[sexusb.slot2.desc.iface] idx=... if=0 class=0x3 subclass=0x0 proto=0x0
[sexusb.slot2.hid.classify] iface=0 role=tablet subclass=0x0 proto=0x0
[sexusb.slot2.hid.role] role=PointerTablet iface=0
[sexusb.slot2.hid.pointer.ready] iface=0
[sexusb.slot2.desc.config] wTotalLength=... interfaces=1 hid_role=PointerTablet
[sexusb.slot2.desc.complete] slot=2 port=6
```

First device path unchanged:
```
[sexusb.hid.bind.summary] keyboard_ep=set pointer_ep=none
[sexusb.intr.classify] kind=keyboard action=forward
```

## Regression Check

| Check | Status |
|-------|--------|
| First device HID bind unchanged | ✅ No code touched |
| First device poll loop unchanged | ✅ No code touched |
| SingleHidBind unchanged | ✅ No code touched |
| `classify_single_hid_role()` reused | ✅ Same function |
| No endpoint config for slot2 | ✅ Not added |
| No SET_CONFIGURATION for slot2 | ✅ Not called |
| Slot2 uses only local variables | ✅ No array refactor |
| Build passes | ✅ |
| HidRole gets Debug (no regression) | ✅ Derive-only, no semantic change |

## Next Phase

**SEXUSB_SECOND_DEVICE_SET_CONFIG_V1**: Issue SET_CONFIGURATION(1) on
slot2's EP0 to select the first (and only) configuration. After SET_CONFIG,
fetch and bind the HID interrupt endpoint for slot2. This requires:

1. SET_CONFIGURATION control transfer (bmReqType=0x00, bReq=0x09,
   wValue=0x0001, wLength=0) on slot2 EP0
2. Wait for device to settle after configuration
3. The config descriptor (already fetched) contains endpoint info —
   but re-fetching after SET_CONFIG to confirm available endpoints
   may be needed
4. Configure Endpoint command for the interrupt-IN endpoint
5. Allocate interrupt ring + report buffer for slot2
6. Start polling slot2 interrupt endpoint

Prerequisite for pointer events reaching sexinput.
