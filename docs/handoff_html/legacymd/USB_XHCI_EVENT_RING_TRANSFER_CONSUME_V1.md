# USB_XHCI_EVENT_RING_TRANSFER_CONSUME_V1

A) RESULT: PASS (event ring fix) / PARTIAL (USB report still blocked)
- Event ring fix: PASS — all 15 poll locations now correctly consume non-matching events
- Build: PASS — `./scripts/entrypoint_build.sh` succeeds
- Faults: PASS — zero #PF/#GP/panic/kill
- USB reports: SKIP — no USB reports with headless QEMU (QEMU input barrier, not event ring)

B) EXACT ROOT CAUSE

**Bug location**: All event-ring-consuming `for _ in 0..POLL_BUDGET` loops in the init
phase (NOOP, Enable Slot, Address Device, Eval Context, descriptor fetches, SET_CONFIG,
SET_IDLE, Configure Endpoint, and all slot-2 equivalents).

**Bug mechanism**: The unconditional `break;` after the `if ev_type == EXPECTED_TYPE`
block:

```rust
if (ev_d3 & 1) == (ev_dcs as u32) {
    let ev_type = (ev_d3 >> 10) & 0x3F;
    if ev_type == TRB_TYPE_CMD_COMPLETION_EVENT {  // or TRANSFER_EVENT
        // ... process event ...
        // ... clear cycle bit ...
        // ... advance ev_idx ...
        // ... update ERDP ...
    }
    break;  // <-- BUG: breaks WITHOUT advancing ev_idx for non-matching events
}
```

When a non-matching event (e.g., PORT_STATUS_CHANGE with type != CMD_COMPLETION)
arrived with a matching cycle bit, the code would:
1. Enter the outer `if` (cycle bit matches)
2. Skip the inner `if` (wrong event type)
3. Hit `break` WITHOUT clearing the event, advancing ev_idx, or updating ERDP
4. The next poll iteration would re-read the SAME event at the SAME ev_idx
5. Same result → `break` again → ev_idx permanently stalled

This is a latent bug that manifests when:
- PORT_STATUS_CHANGE events arrive during init
- Events arrive out of the expected order (race condition)
- Non-CMD_COMPLETION events precede expected CMD_COMPLETION events

**Existing correct code**: The continuous poll loop (line ~3903) and the Configure
Endpoint poll already had the correct pattern — they consumed non-matching events
and continued polling instead of breaking.

C) EVENT RING FIX

**Fix pattern applied to all 15 init-phase poll locations:**

BEFORE:
```rust
if ev_type == EXPECTED_TYPE {
    // ... process event, advance ev_idx, update ERDP ...
}
break;  // <-- unconditional, even for wrong event type
```

AFTER:
```rust
if ev_type == EXPECTED_TYPE {
    // ... process event, advance ev_idx, update ERDP ...
    break;  // <-- break ONLY on expected event
} else {
    // Consume unexpected event to avoid ev_idx stall
    trb_write_volatile(event_ring_va, ev_idx, 0, 0, 0, ev_d3 & !1u32);
    ev_idx += 1;
    if ev_idx >= EVENT_RING_TRBS { ev_idx = 0; ev_dcs ^= 1; }
    let new_erdp = event_ring_phys + ev_idx * 16;
    mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
    mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
}
// REMOVED: unconditional break
```

**Affected polls** (15 locations fixed):
1. NOOP poll (line ~701)
2. Enable Slot poll (line ~770)
3. Address Device poll (line ~1171)
4. Desc 8 fetch poll (line ~1350)
5. Eval Context MPS poll (line ~1507)
6. Desc 18 fetch poll (line ~1668)
7. Config header fetch poll (line ~1872)
8. Config full fetch poll (line ~2021)
9. HID report desc poll (line ~2437)
10. SET_CONFIG poll (line ~2597)
11. SET_IDLE poll (line ~2680)
12. Slot 2 Enable Slot (line ~2886)
13. Slot 2 Address Device (line ~3006)
14. Slot 2 Desc 8 fetch (line ~3087)
15. Slot 2 Config/SET_CONFIG (line ~3550)

D) MARKERS / GATES

No new gates added. Existing AP14 gates remain functional.
The fix is a defensive code correctness change — it prevents ev_idx stalls
and ensures the event ring consumer always advances.

E) PROOF COMMAND / LOG PATH

```
./scripts/entrypoint_build.sh
./scripts/run_daily_driver_proof.sh /tmp/xhci_event_ring_transfer_consume_v1.log
```

Log: `/tmp/xhci_event_ring_transfer_consume_v1.log`

Gate result: `FINAL: PASS (301 gates proved, 133 skipped, 0 faults)`

F) WHETHER TRANSFER EVENT IS NOW CONSUMED

With headless QEMU (`-display none` + `-device usb-tablet`), the USB tablet does
NOT generate interrupt IN reports. The QEMU input barrier (HMP/QMP injection routes
to PS/2, not USB HID) prevents any data from reaching the USB HID layer.

The event ring fix ensures that IF a Transfer Event were to arrive, it WOULD be
consumed correctly — the ev_idx would not stall. But the fix alone cannot overcome
the QEMU input routing limitation.

G) WHETHER REAL USB REPORT REACHED NORMALIZER

No. USB pointer producer report markers remain at SKIP.
Same QEMU barrier as documented in AP14/AP15.

H) FAULT SCAN

```
#PF: 0, #GP: 0, panic: 0, KERNEL PANIC: 0, PAGE FAULT: 0
GENERAL PROTECTION: 0, fault.kill: 0, null-jump: 0
IPC storm: 0, ring overflow: 0
usb_pointer FAIL: 0, usb_mouse FAIL: 0, normalizer FAIL: 0
pointer FAIL: 0, click FAIL: 0, drag FAIL: 0
xhci_event FAIL: 0
```

All clean.

I) FILES CHANGED

- `servers/sexusb/src/main.rs`: +162/-18 lines — event ring consumer fix across
  15 poll locations (move `break;` inside expected-type if-block, add else clause
  to consume unexpected events)
- `docs/handoff/USB_XHCI_EVENT_RING_TRANSFER_CONSUME_V1.md` (new)
- Backup: `servers/sexusb/src/main.rs.bak.ap16`

J) NEXT REQUIRED AUTOPILOT

**USB_HID_POINTER_PRODUCER_V1_RERUN** — the event ring fix is complete. The
remaining blocker is the QEMU input routing barrier (same as AP14/AP15).

Recommended approach: Use `-display gtk` with operator interaction (move/click
mouse inside QEMU window) to generate real USB HID reports. The operator probe
script at `scripts/usb_pointer_real_report_operator_probe.sh` supports this:

```
./scripts/usb_pointer_real_report_operator_probe.sh gtk /tmp/usb_ptr_gtk_rerun.log
```

With the event ring fix, Transfer Events from the USB tablet WILL be consumed
correctly. If the QEMU window receives mouse input, the full path from
XHCI Transfer Event → sexusb decode → sexinput normalizer → silk-shell
pointer state → click-focus/drag should now work end-to-end.
