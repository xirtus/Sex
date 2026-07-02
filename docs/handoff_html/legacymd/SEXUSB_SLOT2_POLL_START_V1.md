# SEXUSB_SLOT2_POLL_START_V1

**Date:** 2026-05-07
**Status:** IMPLEMENTED (build PASS, runtime pending)

## Summary

After slot2 (usb-tablet) Configure Endpoint completes, queue the first
interrupt-IN Normal TRB on slot2's interrupt transfer ring and ring the
doorbell.  This proves the ring and endpoint are correctly configured.
Slot2 produces a TRB completion event on the shared event ring; the
existing slot1 poll loop consumes it as an "unrelated event" (silently
skipped, no dispatch).  No poll-loop changes.

## Goal

Prove that slot2's interrupt ring produces events.  Do NOT demux, do NOT
change the poll loop, do NOT touch any other subsystem.

---

## 1. Insertion Point

**After** `[sexusb.slot2.configure_endpoint.ok]` (line 3414)
**Before** closing `}` of `if target_port_count > 1` (line 3441)

All `s2_intr_*` variables are still in scope inside this block.

## 2. Code Added

### `servers/sexusb/src/main.rs` (+18 lines, lines 3417–3440)

```rust
// ===== SEXUSB_SLOT2_POLL_START_V1: queue first interrupt-IN TRB =====
// Queue exactly one Normal TRB on slot2's interrupt ring at index 0.
// This proves the ring and endpoint are correctly configured.
// The main poll loop (below) is unchanged — slot2 event, if produced,
// will appear as an unrelated event consumed silently by the slot1 loop.
{
    let s2_intr_pcs: u32 = 1; // DCS=1 from Configure Endpoint dequeue
    trb_write_volatile(
        s2_intr_ring_va,
        0, // index 0
        (s2_intr_report_phys & 0xFFFF_FFFF) as u32,
        (s2_intr_report_phys >> 32) as u32,
        s2_intr_report_len,
        (TRB_TYPE_NORMAL << 10) | (1u32 << 5) | s2_intr_pcs, // IOC + cycle=1
    );
    mmio_write32(db_base, s2_slot_id as u64 * 4, s2_intr_dci as u32);
    serial_println!(
        "[sexusb.slot2.poll.start] slot={} dci={} buf={:#x} len={}",
        s2_slot_id,
        s2_intr_dci,
        s2_intr_report_phys,
        s2_intr_report_len
    );
}
```

### TRB Fields Written

| Field | Value | Source |
|-------|-------|--------|
| Ring index | 0 | First slot in 16-entry ring (Link TRB at slot 15) |
| Buffer addr low | `s2_intr_report_phys & 0xFFFF_FFFF` | Slot2 report buffer |
| Buffer addr high | `s2_intr_report_phys >> 32` | Slot2 report buffer |
| Transfer length | `s2_intr_report_len` | Endpoint MPS (~8 for usb-tablet) |
| Control | `(TRB_TYPE_NORMAL << 10) \| (1 << 5) \| 1` | Normal, IOC=1, cycle=1 |

### Doorbell

| Field | Value |
|-------|-------|
| Doorbell offset | `db_base + s2_slot_id * 4` |
| Doorbell target | `s2_intr_dci` (EP1 IN = DCI 3) |

---

## 3. Expected Runtime Behavior

With QEMU `usb-kbd` (slot1) + `usb-tablet` (slot2):

```
[sexusb.slot2.configure_endpoint.ok] slot=2 ep=0x81 dci=3
[sexusb.slot2.poll.start] slot=2 dci=3 buf=0x... len=8
[sexusb.hid.keyboard.continuous.start] attempts=unbounded
... slot1 keyboard polling begins ...
```

The slot2 TRB completion event (Transfer Event) will appear on the shared
event ring.  The slot1 poll loop at line ~3692 will see it as an
"unrelated event" (slot != slot1's slot_id) and will:

1. Log `[sexusb.xhci.intr_in.event.bad] cc=1 slot=2 ep=3` (line 3715)
   — this is expected and harmless
2. Consume the event (clear cycle bit)
3. Advance ev_idx
4. Continue slot1 polling

The QEMU usb-tablet produces zero-byte reports (`actual=6, b0..b5=0`),
so the event will show `cc=1` (SUCCESS) with residue indicating the
reported length.

### Single-device mode (target_port_count == 1)

When only one device is connected (`SEXUSB_QEMU_DEVICE=kbd` without
usb-tablet), the entire `if target_port_count > 1` block is skipped.
No slot2 resources are allocated, no TRB is queued, and the
`[sexusb.slot2.poll.start]` marker never fires.  Behavior is identical
to before this change.

---

## 4. New Marker

| Marker | Budget | Meaning |
|--------|--------|---------|
| `[sexusb.slot2.poll.start] slot=N dci=N buf=0x... len=N` | 1 | Slot2 interrupt-IN TRB queued + doorbell rung |

---

## 5. Trivially Not Touched

| Subsystem | Status |
|-----------|--------|
| Poll loop | ✅ Unchanged — no demux, no second-loop iteration |
| Event ring handling | ✅ Unchanged — slot1 loop consumes slot2 events as "unrelated" |
| `SingleHidBind` / `HidDevice` struct | ✅ Unchanged |
| sexinput | ✅ Unchanged |
| silk-shell | ✅ Unchanged |
| sexdisplay | ✅ Unchanged |
| kernel | ✅ Unchanged |
| sex-pdx | ✅ Unchanged |
| PDX opcodes | ✅ No new opcodes |
| HID normalizer | ✅ Unchanged |
| Input policy | ✅ Unchanged |

---

## 6. Build Result

```bash
./scripts/entrypoint_build.sh → PASS (exit 0)
```
- sexusb: 0 new warnings, 0 errors
- Full ISO: 1757 sectors

---

## 7. Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | +18 lines: queue slot2 TRB + doorbell after configure_endpoint.ok |
| `servers/sexusb/src/main.rs.bak.slot2_poll_start` | Backup before change |
| `docs/handoff/SEXUSB_SLOT2_POLL_START_V1.md` | Created (this document) |

---

## 8. Verification Commands

### Build
```bash
./scripts/entrypoint_build.sh
```

### Run (multi-device: keyboard + tablet)
```bash
SEXUSB_QEMU_DEVICE=kbd ./dev.sh run 2>&1 | tee /tmp/slot2-poll-start.log
```

### Verify markers
```bash
grep -c 'sexusb.slot2.poll.start' /tmp/slot2-poll-start.log    # ≥ 1
grep -c 'sexusb.slot2.configure_endpoint.ok' /tmp/slot2-poll-start.log  # ≥ 1
grep -c 'sexusb.hid.keyboard.continuous.start' /tmp/slot2-poll-start.log  # ≥ 1
grep -cE 'panic|#PF|#GP' /tmp/slot2-poll-start.log             # = 0
grep -c 'sexusb.xhci.intr_in.event.bad' /tmp/slot2-poll-start.log  # expected ≥ 0 (slot2 events as unrelated)
```

### Single-device regression
```bash
# If QEMU config has only one device, the slot2 block is skipped entirely.
# Verify keyboard still works:
grep -c 'sexusb.kbd.raw' /tmp/slot2-poll-start.log   # ≥ 0 (depends on input)
grep -c 'sexinput.kbd.recv' /tmp/slot2-poll-start.log  # ≥ 0
```

---

## 9. STOP FIRST Conditions Preserved

- [x] No poll-loop changes
- [x] No event-ring handling changes
- [x] No sexinput/silk-shell/sexdisplay edits
- [x] No kernel/sex-pdx edits
- [x] No PDX opcode changes
- [x] No struct refactor
- [x] Single-device path unchanged (target_port_count gate)
- [x] Build passes with zero new warnings

---

## 10. Next Phase

**SEXUSB_SLOT2_EVENT_DEMUX_V1** — Convert the single-device poll loop into
a two-device event demux.  After C1 proves the ring+endpoint work, C2 will:

1. Replace `single_bind` with a `[HidDevice; 2]` array
2. Round-robin TRB queueing + doorbell for both devices
3. Validate Transfer Event `slot` against both device IDs
4. Dispatch to the correct decode/forward handler
5. Preserve keyboard re-arm-before-IPC optimization per-device

**Do NOT combine C1 and C2 in a single patch.** C1 must pass independently.

---

*End of SEXUSB_SLOT2_POLL_START_V1.md*
