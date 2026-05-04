# PHASE 08: Network + Device Reality Pass

## Goal
Make networking and devices visible and usable. sexnet status UI, network route nodes in Mesh, sexusb device catalog (after Phase 3B succeeds), device class routing from sexusb to sexinput/sexaudio/sexdrive.

## Ownership
- **sexnet** (exclusive): network stack, status, routes, connection state
- **sexusb** (exclusive): USB device catalog, class routing
- **Mesh** (consumer): network + device node visibility (Phase 6 graph)
- **sexinput** (consumer): receives HID devices from sexusb
- **sexaudio** (future): receives audio devices from sexusb
- **sexdrive/sexfiles** (future): receives storage devices from sexusb
- **Quil** (consumer): network/device management panels

## What Already Exists
- `sexusb` server exists, boots, listens on PDX slot
- `sexnet` defined in manual as network stack server
- No network stack implementation exists
- No USB device catalog exists (Phase 3B must complete first)
- Mesh graph model (Phase 6) provides visualization for network+device nodes
- SilkBar has a clock chip but no network status chip

## Bundle

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| sexnet status UI | Connection state, IP, link status as Quil panel | 6h | High |
| sexnet → Mesh push | sexnet pushes network state to Mesh graph nodes | 2h | Medium |
| sexusb device catalog | Enumerated USB devices with descriptors exposed via PDX | 4h | Medium (after Phase 3B) |
| Device class routing | sexusb → sexinput (HID), sexusb → sexaudio (future), sexusb → sexdrive (future) | 4h | Medium |
| SilkBar network chip | Show connection status in system bar | 2h | Low |
| No sexlink monolith | sexusb is USB-only bus owner; future sexlink is catalog only | Documentation | Always |

## Smallest First Step
Get sexnet to report link status (up/down) and local IP via PDX. No routing, no DNS, no TCP — just "network exists, here's my address." This can be tested with QEMU's `-netdev user` mode. Proving the network interface is visible is the foundation for all network features.

## Dependencies
- **Blocking**: Phase 3B (USB device catalog needs real USB enumeration working)
- **Blocked by**: Phase 3B for sexusb device catalog; independent for sexnet UI
- **Can parallelize with**: Phase 7 (app launch), Phase 9 (Bell/settings)
- **Key insight**: sexnet UI and sexusb device catalog are fully independent. sexnet is a network service, sexusb is a hardware service. They share nothing except Mesh visualization.

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Network stack is complex (TCP/IP requires significant code) | High | HIGH | V1: Only link status + IP address. No TCP/IP stack in this phase. Use ICMP echo or raw Ethernet for proof of connectivity. Defer TCP to a later phase. |
| sexnet needs kernel network syscalls | High | High | If kernel doesn't have network stack, sexnet must implement it in userspace using PDX to access the NIC. Start with QEMU's virtio-net via MMIO. |
| USB class routing to sexinput/sexaudio/sexdrive is complex | Medium | Medium | V1: sexusb exposes device descriptors. Consumer servers poll or register interest. No automatic routing — explicit forwarding via PDX. |
| Device catalog grows unbounded | Low | Low | Fixed-size device table (max 16 devices). Reject beyond that. Real systems don't have 16+ USB devices simultaneously. |

## Exit Criteria (Done Checklist)
- [ ] sexnet reports link status (up/down) via PDX
- [ ] sexnet reports local IP address
- [ ] sexnet pushes network state to Mesh as graph nodes
- [ ] sexusb device catalog PDX call returns list of connected devices (after Phase 3B)
- [ ] HID device from sexusb reaches sexinput and silk-shell (proving class routing)
- [ ] Quil network panel shows link status and IP
- [ ] Quil device panel shows USB device list
- [ ] Build passes. Boot passes. No panic.

## Testing Strategy
- **sexnet**: Boot QEMU with `-netdev user,id=net0 -device virtio-net,netdev=net0`. Verify sexnet detects link up and gets IP from DHCP (or hardcoded IP).
- **Device catalog**: After Phase 3B, boot with USB mouse. Verify sexusb device catalog returns mouse descriptor.
- **Integration**: Verify network node appears in Mesh graph. Verify USB device appears in Mesh graph.
- **Regression**: All existing shell/display/input markers fire.

## Efficiency Opportunity
**sexnet V1 doesn't need a TCP/IP stack.** A "network is up" boolean and an IP address display is sufficient for Phase 8. Real networking (browsing, file transfer) requires TCP/IP which is a major engineering effort. **Defer TCP/IP to Phase 11 or later.** Phase 8 is about observability, not connectivity.

## Completeness Gain
Networking/devices: **40–55% → 60–70%** (with observability only). Full network stack deferred.

## Files Changed
- `servers/sexnet/src/main.rs` (link status PDX, IP reporting, Mesh push)
- `servers/sexusb/src/main.rs` (device catalog PDX call, after Phase 3B)
- `servers/quil/src/main.rs` (network panel, device panel)
- `crates/sex-pdx/src/lib.rs` (OP_NET_STATUS, OP_USB_DEVICE_CATALOG opcodes)

## Forbidden
- Full TCP/IP stack (deferred — V1 is link status only)
- sexlink monolith (sexusb stays narrow)
- USB/XHCI rewrite (only add device catalog on top of Phase 3B)
- Kernel network stack changes (userspace networking only)

## Next Phase
PHASE_09_BELL_NOTIFICATIONS_SETTINGS.md
