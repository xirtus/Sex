# SEXUSB_MULTIDEVICE_PORT_SCAN_V1

## Status: IMPLEMENTED

## Summary

Replaced the port-scan `break` on first connected port with a bounded
`target_ports[]` array.  Up to `MAX_USB_DEVICES` (2) connected ports are
collected during the initial PORTSC scan.  Downstream behavior is
unchanged: only `target_ports[0]` is used as the active device target.

---

## Constants

```rust
const MAX_USB_DEVICES: usize = 2;
```

Chosen because the current QEMU test configuration has exactly two
devices (keyboard + tablet).  Can be increased in a future phase when
multi-device enumeration and event demux are implemented.

---

## Collection Logic

```rust
let mut target_ports: [u8; MAX_USB_DEVICES] = [0; MAX_USB_DEVICES];
let mut target_port_count: usize = 0;

for port in 1..=max_ports {
    let portsc = mmio_read32(op_base, portsc_off);
    // ... log all ports, count ports_connected ...
    if (portsc & PORTSC_CCS) != 0 {
        if target_port_count < MAX_USB_DEVICES {
            target_ports[target_port_count] = port as u8;
            if target_port_count == 0 {
                port_speed = speed;  // first device speed (for EP0)
            }
            target_port_count += 1;
        }
    }
}
```

- **No `break`**: The full port scan iterates all ports, logging each one.
  Connected ports are collected into the array up to `MAX_USB_DEVICES`.
- **First device speed**: `port_speed` is captured from the first
  connected port for EP0 MPS detection (unchanged from before).
- **Overflow guard**: If `ports_connected > MAX_USB_DEVICES`, the driver
  halts with `[sexusb.ports.overflow]`.  This is a safety gate: the
  downstream code cannot handle more devices yet.

---

## Downstream Compatibility

After the scan loop, `target_port` is computed as a scalar u64:

```rust
let target_port: u64 = target_ports[0] as u64;
```

All downstream code (slot context, ICC audit, port reread, MPS detection,
SingleHidBind creation) continues to use `target_port` as before.  No
downstream line was modified.

| Downstream site | Uses | Unchanged |
|----------------|------|-----------|
| Slot context DW1 `(target_port as u32) << 16` | ✅ | u64 → u32 cast |
| ICC audit reread `target_port - 1` | ✅ | u64 arithmetic |
| ICC audit log `target_port as u32` | ✅ | identity |
| Slot log `port={}` | ✅ | identity |
| `SingleHidBind { port: target_port }` | ✅ | identity |

---

## Markers

| Marker | Purpose |
|--------|---------|
| `[sexusb.ports.collect] count=N first=N` | Emitted once after scan: reports how many ports were collected and the first port number |
| `[sexusb.ports.overflow] count=N max=N` | Emitted (then halt) if more than MAX_USB_DEVICES devices connected |

---

## Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | Port scan: `target_ports[]` array replaces scalar `target_port` + `break`; `MAX_USB_DEVICES` constant; overflow guard; collection marker |
| `docs/handoff/SEXUSB_MULTIDEVICE_PORT_SCAN_V1.md` | Created |

---

## Build

```
./scripts/entrypoint_build.sh: PASS
```

---

## Next Phase Prompt

```
MISSION: SEXUSB_SECOND_SLOT_ENABLE_V1
Goal: Enable a second XHCI slot for target_ports[1] after the first
device's SET_CONFIGURATION completes.  Do NOT poll the second device
yet — only ensure the slot exists and the device is addressed.

Acceptance:
  - After first device reaches continuous poll loop, issue Enable Slot
    and Address Device for target_ports[1]
  - Second device uses target_ports[1] port number
  - Log [sexusb.slot2.enable] and [sexusb.slot2.address] markers
  - Do not configure endpoints or poll second device yet
  - Build passes
  - Single-device behavior identical when only 1 port connected
```
