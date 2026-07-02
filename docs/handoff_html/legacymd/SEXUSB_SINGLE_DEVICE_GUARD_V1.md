# SEXUSB_SINGLE_DEVICE_GUARD_V1

## Status: COMPLETE (comments only, no logic change)

## Summary

Added in-code doc comments at every structural chokepoint in
`servers/sexusb/src/main.rs` that would need to change for multi-device
HID support.  No runtime behavior changed.  The single-device limitation
is now impossible to miss during future edits.

---

## Comment Locations

| Location | Lines | What It Documents |
|----------|-------|-------------------|
| Module header | 6–17 | Top-level limitation: port scan break, single slot, single bind, single poll loop |
| `SingleHidBind` struct | 196–199 | Must become array; each device needs own slot/ring/buffer |
| Port scan `break` | 767–780 | Do not remove break without multi-slot/bind/ring/demux |
| HID role classification | 2178–2189 | Priority keyboard>tablet>mouse; if first device is keyboard, tablet decoder is unreachable |
| `single_bind` init | 2201–2205 | Only one binding created; needs array for Phase 3+4 |
| Continuous poll loop | 2781–2798 | Single-device loop; serves one endpoint; needs event demux for multi-device |
| Tablet decode entry | 3070–3078 | Complete and correct but unreachable when keyboard wins enumeration |
| Mouse decode entry | 3188–3189 | Same single-device limitation |

---

## Future Phase Sequence

### Phase 2: Collect connected ports
Replace the `break` with a `target_ports[]` array up to `MAX_USB_DEVICES`.
No functional change until downstream code iterates the array.

**File**: `servers/sexusb/src/main.rs` (port scan loop)
**Risk**: Low.

### Phase 3: Multi-slot device enumeration
Extract the single-device enumeration pipeline
(Enable Slot → Address Device → GET_DESCRIPTOR → HID walk → SET_CONFIG →
Configure Endpoint) into a reusable helper, then call it per port.

**Structural changes needed**:
- `SingleHidBind` → `[Option<HidDevice>; MAX_HID_DEVICES]`
- Each `HidDevice` owns its `slot_id`, `intr_ring_va/phys`,
  `intr_report_va/phys`, `intr_prod`, `intr_pcs`
- Descriptor data buffer shared but serialized (one device at a time)

**Sub-phase breakdown**:
1. Extract Enable Slot + Address Device as `fn enable_device(port) -> SlotId`
2. Extract GET_DESCRIPTOR + HID walk as `fn identify_role(slot) -> HidRole`
3. Extract Configure Endpoint + interrupt setup as `fn start_poll(dev: &mut HidDevice)`

### Phase 4: Event demux poll loop
Replace the single `loop { wait; handle; rearm }` with an event ring
demux that matches Transfer Event `(slot, ep)` against registered devices.

**Key design constraints**:
- Keyboard re-arm-before-IPC optimization (line ~2941) must be preserved
  or generalized: any device may need to re-arm before `pdx_call` to avoid
  lost interrupts during IPC blocking
- The `skip_advance` mechanism (line 2790) must work per-device, not globally
- No device's interrupt polling should be starved while another device's
  IPC is in flight

### Alternative: Keyboard Cursor Fallback
Instead of multi-device USB, `KEYBOARD_CURSOR_ENABLED` uses arrow/WASD
keys to synthesize relative motion events via the keyboard path.  This
bypasses the USB multi-device limitation entirely for keyboard-driven
pointer control.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | Added in-code guard comments (see table above) |
| `docs/handoff/SEXUSB_SINGLE_DEVICE_GUARD_V1.md` | Created |

---

## Build

```
./scripts/entrypoint_build.sh: PASS
```

## Acceptance Checklist

- [x] No runtime logic changed
- [x] No `break` removed, no arrays added, no HID decode changed
- [x] No sexinput/silk-shell/sexdisplay/kernel/QEMU touched
- [x] Single-device limitation impossible to miss during edits
- [x] Build passes
- [x] Backup created at `main.rs.bak.single_device_guard`

## Next Implementation Prompt

```
MISSION: SEXUSB_MULTIDEVICE_PORT_SCAN_V1
Goal: Replace port-scan break with target_ports[] array.
Acceptance:
  - target_ports[] collects up to MAX_USB_DEVICES connected ports
  - If only 1 port connected, behavior identical to current break
  - error if >MAX_USB_DEVICES connected (log and hang)
  - no downstream code iterates the array yet
  - Build passes
```
