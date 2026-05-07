# SEXUSB_INPUT_COMPLETION_PLAN_V1

**Date:** 2026-05-07
**Status:** PLAN — Phase-gated audit with implementation prompts
**Scope:** Plan-only. No code changes.

---

## 1. Executive Summary

The SexOS guest input pipeline is **proven correct** end-to-end for all software
paths. The blocker to real USB pointer/trackpad/mouse input is a **QEMU host-side
routing gap** — QEMU 11.0.0 on this host does not bridge host pointer events
(mouse movement, tablet absolute coordinates) into the emulated USB HID report
buffer. The guest pipeline (xHCI → sexusb → sexinput → silk-shell → sexdisplay)
works perfectly when data arrives.

This plan defines a phased approach to close the gap: first by making sexusb
capable of polling TWO devices simultaneously (the missing software piece),
then by identifying the data-source path (QMP injection / QEMU fix / hardware
boot) that delivers real pointer data through the proven pipeline.

---

## 2. Current Status — What Is Proven

### 2.1 Proven Paths (GREEN)

| Path | Proof Markers | Handoff | Notes |
|------|---------------|---------|-------|
| XHCI controller discovery | `[usb.host.controller.found]` `[usb.host.caps]` | USB_HOST_DISCOVERY_V1 | ✅ MMIO BAR mapped, caps readable |
| Single-device XHCI enumeration | Enable Slot → Address Device → GET_DESCRIPTOR → SET_CONFIG → Configure Endpoint | SEXUSB_SECOND_*_V1 set | ✅ Works for both slot 1 and slot 2 |
| USB keyboard HID raw reports | `[sexusb.kbd.raw]` → `[sexinput.kbd.recv]` | KEYBOARD_DEVICE_MODE_V1 | ✅ Real key data arrives from QEMU |
| USB keyboard → cursor fallback | `[keyboard_cursor.key]` → `[shell.cursor.move]` | KEYBOARD_DEVICE_MODE_V1 | ✅ WASD/arrow moves cursor |
| PS/2 keyboard → EV_KEY | `[input.proof.keyboard.recv]` → `[shell.key.ev_key.received]` | KEYBOARD_EDGE_PROOF_V1, INPUT_REAL_DEVICE_RELIABILITY_V1 | ✅ Enter keydown/up chain |
| Synthetic pointer drag (242 frames) | `[sexusb.synthetic.drag.*]` → `[sexdisplay.cursor.draw]` | INPUT_PHASE_CLOSEOUT_V1 | ✅ Full end-to-end, zero panics |
| Synthetic click → focus | `[shell.click_focus.down/hit/send.ok]` | INPUT_CLICK_FOCUS_PROOF_V1, REAL_CLICK_TARGET_PROOF_V1 | ✅ EV_ABS anchor + BTN down/up |
| sexinput normalizer (pointer) | `normalize_pointer_report_v1` — button edge, EV_REL, EV_ABS | INPUT_REAL_DEVICE_RELIABILITY_V1 | ✅ Works for any 3-byte report |
| sexinput → silk-shell HID route | EV_KEY/EV_REL/EV_ABS/EV_BTN → OP_HID_EVENT(0x202) | SEXINPUT_TO_SHELL_ROUTE_AUDIT_V1 | ✅ All event classes |
| silk-shell click focus + drag state machine | `shell.interact.drag.begin/move/end` | INPUT_CLICK_FOCUS_PROOF_V1 | ✅ Full state machine wired |
| sexdisplay cursor draw | `[sexdisplay.cursor.draw]` | INPUT_PHASE_CLOSEOUT_V1 | ✅ Cursor surface tracked |
| Input ownership rules | EV_REL owns movement, EV_BTN owns clicks, EV_ABS owns positioning | REAL_CLICK_TARGET_PROOF_V1 | ✅ No double-apply |
| Multi-device port scan | `target_ports[]` array, up to MAX_USB_DEVICES=2 | SEXUSB_MULTIDEVICE_PORT_SCAN_V1 | ✅ Both ports collected |
| Slot2 enumeration | Enable Slot → Address → 4x GET_DESCRIPTOR → HID classify → SET_CONFIG → Configure Endpoint | SEXUSB_SECOND_*_V1 (5 docs) | ✅ All phases complete |

### 2.2 Not-Proven (Gaps)

| Gap | Reason | Blocker Level |
|-----|--------|---------------|
| **Real USB pointer data** (mouse/tablet motion) | QEMU 11.0.0 does not route host pointer events to emulated USB HID report buffer | Host-side (QEMU) |
| **Slot2 continuous polling** | Poll loop at line 3647 is single-device — only polls `single_bind` | Guest-side (sexusb) |
| **Multi-device event demux** | Event ring consumption matches single `slot_id` only (line 3686) | Guest-side (sexusb) |
| **Real hardware boot input** | USB boot + physical hardware never tested | Test gap |
| **QEMU HID report buffer bridge** | `hw/usb/dev-hid.c` in QEMU source not connecting host events to USB HID | Host-side (QEMU) |

---

## 3. Root Cause Analysis

### 3.1 Guest Pipeline: Proven Working

Every software layer between "HID report bytes arrive in guest memory" and
"cursor moves on screen" is proven:

```
HID report bytes in intr_report_va
  → sexusb: decode_tablet_report() / decode_mouse_report()
  → pdx_call(SLOT_USB_SEXINPUT, OP_USB_MOUSE_REPORT)
  → sexinput: normalize_pointer_report_v1()
  → pdx_call(SLOT_SHELL, OP_HID_EVENT, EV_BTN/EV_REL/EV_ABS)
  → silk-shell: EV_BTN handler → hit-test → focus change
  → silk-shell: EV_REL handler → POINTER_X/Y update
  → pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_CURSOR)
  → sexdisplay: cursor surface at new position
```

Proven via synthetic drag proof (242 frames, all markers pass, zero panics).

### 3.2 Data Source Gap: QEMU Host Input → USB HID

QEMU 11.0.0 on this host produces zero-byte USB HID reports for all pointer
devices (usb-mouse, usb-tablet). Evidence from USB_CURSOR_ROUTE_PROOF_V1:

```
[sexusb.hid.tablet.raw] b0=0x0 b1=0x0 b2=0x0 b3=0x0 b4=0x0 actual=6
```

- `actual=6` proves the interrupt-IN transfer completes
- All 6 bytes are zero — the report buffer was never populated
- The QEMU internal pipeline: `host input (SDL/GTK) → QEMU HID Tablet handler → USB HID report buffer` is broken
- The XHCI controller + USB protocol layers work (transfer completes)
- The HID emulation layer (`hw/usb/dev-hid.c`) is the suspected break

USB **keyboard** HID reports DO carry real data. The gap is specific to
pointer/tablet devices.

### 3.3 Software Gap: Single-Device Poll Loop

Even if QEMU started delivering pointer data tomorrow, **sexusb only polls
one device**. The poll loop at line 3647 in `servers/sexusb/src/main.rs`:

```rust
loop {
    // Queue ONE Normal TRB on intr_ring for single_bind.slot_id
    // Ring doorbell for single_bind.slot_id
    // Wait for Transfer Event with slot == single_bind.slot_id
    // Decode and forward report from this one device
}
```

Slot2 has its interrupt ring + report buffer allocated and its interrupt
endpoint configured via Configure Endpoint command. But **nothing ever queues
a TRB on slot2's interrupt ring** or **rings slot2's doorbell**. Slot2's
TRB ring sits idle with no pending transfers.

---

## 4. Phase Plan

### Phase A — USB_HOST_DISCOVERY_AUDIT (COMPLETE ✅)

**Status:** Done. XHCI controller discovered, BAR mapped, capability registers
readable. Port scan collects both connected ports (usb-kbd on port 5,
usb-tablet on port 6). MAX_USB_DEVICES=2.

**Handoffs:** USB_HOST_DISCOVERY_V1, SEXUSB_MULTIDEVICE_PORT_SCAN_V1

**Markers (all verified):**
```
[usb.host.discovery.start]
[usb.host.controller.found] slot=8 ...
[usb.host.caps] caplength=.. hciversion=.. ...
[sexusb.ports.collect] count=2 first=5
[sexusb.xhci.addr_ctx.ports] 8
```

### Phase B — XHCI_ENDPOINT_PROOF (COMPLETE ✅)

**Status:** Done. Both slot1 (usb-kbd) and slot2 (usb-tablet) go through full
XHCI enumeration: Enable Slot → Address Device → GET_DESCRIPTOR → HID classify →
SET_CONFIGURATION → Configure Endpoint.

**Slot1 (usb-kbd):** Configured, polling, delivering keyboard reports.
**Slot2 (usb-tablet):** Configured, NOT polling. Interrupt ring + report buffer
allocated, endpoint context set, but no TRB queued and no doorbell rung.

**Handoffs:** SEXUSB_SECOND_SLOT_ENABLE_V1, SEXUSB_SECOND_DEVICE_GET_DESCRIPTOR_V1,
SEXUSB_SECOND_HID_ROLE_BIND_V1, SEXUSB_SECOND_DEVICE_SET_CONFIG_V1,
SEXUSB_SECOND_DEVICE_CONFIGURE_ENDPOINT_V1

**Markers (expected at runtime for slot2):**
```
[sexusb.slot2.enable.ok] slot=2
[sexusb.slot2.address.ok] slot=2 port=6
[sexusb.slot2.desc.complete] slot=2 port=6
[sexusb.slot2.hid.role] role=PointerTablet iface=0
[sexusb.slot2.hid.pointer.ready] iface=0
[sexusb.slot2.set_config.ok] slot=2
[sexusb.slot2.configure_endpoint.ok] slot=2 ep=1 dci=3
```

### Phase C — HID_REPORT_FETCH_PROOF (NEXT ⬇️)

**Status:** NOT DONE. This is the first new phase.

**Goal:** Start continuous interrupt-IN polling for slot2's usb-tablet device.
Convert the single-device poll loop into a two-device event demux.

**Sub-phases:**

#### C1 — Queue First TRB + Ring Doorbell for Slot2

Before entering the poll loop, after slot2 Configure Endpoint completes:
1. Read slot2's EP0 dequeue pointer to verify EP0 is clean
2. Queue one Normal TRB on `s2_intr_ring_va` at index 0:
   - Data buffer = `s2_intr_report_phys`
   - Length = `s2_intr_report_len` (read from endpoint descriptor, typically 8)
   - IOC=1, cycle = `s2_intr_pcs` (starts at 1)
3. Ring doorbell for slot2 interrupt-IN endpoint:
   - `mmio_write32(db_base, s2_slot_id * 4, s2_intr_dci)`
4. Log marker: `[sexusb.slot2.poll.start] slot=N dci=N`

**Risk:** Low. Single TRB write + MMIO write. No loop changes.
**Lines:** ~15 new lines after Configure Endpoint.
**Fallback:** If TRB queues but no data arrives (QEMU delivers zero reports),
the poll loop in C2 will still work — it just gets idle (zero-byte) reports.

#### C2 — Event Demux in Poll Loop

Convert the poll loop (line 3647) to handle two devices:

**Current structure:**
```rust
loop {
    queue TRB on intr_ring for single_bind.slot_id
    ring doorbell for single_bind.slot_id
    wait for Transfer Event with slot == single_bind.slot_id
    decode + forward report
    advance intr_prod
}
```

**Target structure:**
```rust
loop {
    // Round-robin: alternate between device 0 (slot1) and device 1 (slot2)
    for dev_idx in 0..active_devices {
        let dev = &devices[dev_idx];
        queue TRB on dev.intr_ring
        ring doorbell for dev.slot_id * 4, dev.intr_dci
        // Wait for ANY Transfer Event
        let (slot, ep, cc) = wait_transfer_event(...);
        // Dispatch to matching device
        if slot == dev.slot_id && ep == dev.intr_dci {
            decode + forward report for this device
            advance dev.intr_prod
        } else if slot == other_dev.slot_id {
            // Process other device's event (bonus: it arrived while we were
            // waiting for this device)
            decode + forward for other device
            advance other_dev.intr_prod
        }
    }
}
```

**Design constraints:**
- Keyboard re-arm-before-IPC optimization (lines 3775-3840) must be preserved
- `skip_advance` mechanism must work per-device
- No device starves while another's `pdx_call` IPC is in flight
- All event ring consumption must validate `slot` against both device IDs

**Risk:** Medium. The poll loop is ~400 lines with subtle re-arm timing.
Extracting device state into a struct array is the right refactor.

**Lines changed:** ~80-120 in the poll loop block.

#### C3 — Validate Zero-Byte Reports (Idle Path)

With QEMU 11.0.0 delivering zero-byte tablet reports, the demux will initially
only prove that slot2 polling WORKS — not that pointer data arrives. This is
still valuable: it proves the multi-device architecture, event demux, and
re-arm logic are correct.

Acceptance for C3:
- Slot2 interrupt-IN TRB completes with `actual=6` (same as current behavior)
- Event demux correctly dispatches to slot2's handler
- No event-ring corruption (wrong slot/cycle-bit consumption)
- Slot1 keyboard polling continues uninterrupted
- Build passes, zero panics

**Markers for Phase C:**
```
[sexusb.slot2.poll.start] slot=2 dci=3
[sexusb.slot2.poll.intr] n=0 actual=6
[sexusb.slot2.poll.demux] slot=2 ep=1 cc=1
[sexusb.demux.active] devices=2
```

### Phase D — HID_NORMALIZE_PROOF (ALREADY PROVEN ✅)

**Status:** Done. sexinput's `normalize_pointer_report_v1` handles any 3-byte
boot mouse report. When non-zero pointer data arrives (from any source), the
normalizer produces correct EV_BTN, EV_REL, and EV_ABS events.

**No new work needed.** The normalizer is source-agnostic.

### Phase E — POINTER_TO_SHELL_PROOF (ALREADY PROVEN ✅)

**Status:** Done. silk-shell's EV_REL handler updates POINTER_X/Y, EV_BTN handler
processes click-focus, EV_ABS handler anchors position. All proven via synthetic
drag proof.

**No new work needed.** The shell handlers are source-agnostic.

### Phase F — CLICK_FOCUS_REAL_DEVICE_PROOF (DEPENDS ON DATA SOURCE)

**Status:** NOT DONE. Requires non-zero pointer data reaching sexinput.

**Goal:** Prove a real click-focus chain with non-zero pointer data from a
real device (QEMU tablet/mouse or real hardware).

**Sub-phases:**

#### F1 — QMP Pointer Injection (if QMP works for pointer)

Use `scripts/qmp_input_probe.py` (or a new pointer-specific variant) to inject
absolute pointer events via QEMU QMP:

```fish
env SEXOS_QEMU_QMP=1 SEXUSB_QEMU_DEVICE=tablet ./dev.sh run &
./scripts/qmp_input_probe.py /tmp/sexos-qmp.sock
```

**Acceptance:**
- `[sexusb.hid.tablet.nonzero.ok]` fires (first non-zero report after idle)
- `[sexinput.mouse.real.delta]` fires with non-zero dx/dy
- `[shell.cursor.move]` fires with changing x/y coordinates
- `[sexdisplay.cursor.draw]` fires at new positions
- All 6 INPUT_REAL_DEVICE_RELIABILITY markers pass with non-zero values

**Risk:** QMP injection of pointer events may also produce zero reports on this
QEMU/host combo (same QEMU HID layer bug). If so, fall through to F2 or F3.

#### F2 — Real Hardware Boot

Boot SexOS on real hardware with USB keyboard + mouse connected:
1. Build ISO with default gate (no synthetic, real USB path)
2. Boot via USB stick or PXE
3. Capture serial log
4. Verify all pointer markers fire with non-zero data

**Risk:** Requires physical hardware, UEFI boot setup, and serial capture.
No code changes needed — the guest pipeline is already correct.

#### F3 — QEMU Source Fix (Track A)

Modify patched QEMU's `hw/usb/dev-hid.c` to bridge host input events into
the USB HID report buffer. This requires QEMU C expertise and is outside
the SexOS guest scope.

**Risk:** Medium (QEMU source changes). **Recommended for a separate session.**

**Markers for Phase F:**
```
[sexusb.hid.tablet.nonzero.ok] i=N buttons=0x1 x=6400 y=7200
[sexinput.mouse.real.delta] dx=5 dy=-3 buttons=0x1
[sexinput.pointer.button.down] btn=1 pressed=true
[sexinput.pointer.button.up] btn=1 pressed=false
[silk-shell.pointer.recv] class=EV_ABS x=640 y=360
[silk-shell.pointer.recv] class=EV_BTN btn=1 pressed=true
[silk-shell.click.down] btn=1 x=640 y=360
[silk-shell.focus.change] from=0 to=201
[shell.interact.drag.begin]
[shell.interact.drag.move]
[shell.interact.drag.end]
[shell.cursor.move] x=... y=...
```

---

## 5. Exact Marker Definitions Per Phase

### Phase C Markers (HID_REPORT_FETCH_PROOF)

| Marker | Budget | Location | Meaning |
|--------|--------|----------|---------|
| `[sexusb.slot2.poll.start] slot=N dci=N` | 1 | After C1 TRB + doorbell | Slot2 interrupt polling started |
| `[sexusb.slot2.poll.intr] n=N actual=N` | 16 | C2 demux, after TRB completion for slot2 | Slot2 HID report received |
| `[sexusb.slot2.poll.demux] slot=N ep=N cc=N` | 16 | C2 demux dispatch | Event matched to slot2 |
| `[sexusb.demux.active] devices=N` | 1 | C2 poll loop entry | How many devices being polled |
| `[sexusb.demux.dispatch] slot=N kind=tablet|keyboard` | 32 | C2 per-event dispatch | Which device handler invoked |
| `[sexusb.slot2.poll.idle] i=N` | unbounded (HID_VERBOSE_RING_LOG) | C2 idle report path | Zero-byte tablet report |

### Phase F Markers (CLICK_FOCUS_REAL_DEVICE_PROOF)

| Marker | Budget | Location | Meaning |
|--------|--------|----------|---------|
| `[sexusb.hid.tablet.nonzero.ok] i=N buttons=N x=N y=N` | 1 | sexusb tablet decode | First non-zero tablet report |
| `[sexinput.mouse.real.delta] dx=N dy=N buttons=N` | 16 | sexinput real path | Non-zero delta received (existing) |
| `[sexinput.pointer.button.down] btn=1 pressed=true` | 16 | sexinput forward path | Left button press (existing) |
| `[shell.cursor.move] x=N y=N` | 16 | silk-shell cursor update | Cursor moved (existing) |
| `[shell.click_focus.down] x=N y=N buttons=N` | 16 | silk-shell click handler | Click target hit-test (existing) |
| `[shell.click_focus.hit] id=N` | 16 | silk-shell click handler | Surface hit by click (existing) |
| `[silk-shell.focus.change] from=N to=N` | 8 | silk-shell focus handler | Focus transferred (existing) |
| `[sexdisplay.cursor.draw] n=N x=N y=N` | 16 | sexdisplay render | Cursor drawn at new position (existing) |

---

## 6. STOP FIRST Boundaries

### Absolute STOP (do not proceed under any circumstances)

1. **Kernel edits.** No `kernel/src/` changes for input plumbing.
2. **sex-pdx ABI changes.** No new opcodes without explicit STOP FIRST review.
   `OP_USB_MOUSE_REPORT(0x260)` and `OP_HID_EVENT(0x202)` are sufficient.
3. **sexdisplay/silk-shell policy changes.** No compositor, hit-test, focus,
   or interaction policy changes in this plan.
4. **Broad refactor.** No extraction of the ~4000-line sexusb `main()` into
   modules. Phase C2's refactor is scoped to the poll loop only.
5. **Gestures or multi-touch before click/focus.** No gesture recognition,
   multi-touch, or scroll handling before basic click-focus is proven.
6. **Storage/Linen/sexfiles edits.** No cross-subsystem changes.
7. **All-at-once patches.** Never combine USB polling + HID normalization +
   shell policy + display rendering in one commit.

### Conditional STOP (pause and re-read handoffs)

1. If a change touches both sexusb poll loop AND sexinput normalizer in the
   same patch → split into two patches.
2. If event-ring corruption appears (wrong cycle bit, stale TRB) → STOP,
   audit event ring consumption order before continuing.
3. If first-device (slot1) behavior regresses → STOP, verify single-device
   path before adding slot2.
4. If `pdx_call` blocking causes starved interrupts on the other device →
   STOP, redesign re-arm ordering before proceeding.

---

## 7. First Implementation Prompt — Phase C1

```
MISSION: SEXUSB_SLOT2_POLL_START_V1

Goal:
Queue the first interrupt-IN Normal TRB for slot2 (usb-tablet) and ring its
doorbell BEFORE the main poll loop. Do NOT change the poll loop yet. This
proves that slot2's interrupt ring and endpoint are correctly configured.

Context:
- slot2 has been fully enumerated: Enable Slot, Address Device,
  GET_DESCRIPTOR, HID classify (PointerTablet), SET_CONFIGURATION,
  Configure Endpoint.
- Slot2 resources: s2_intr_ring_va/phys, s2_intr_report_va/phys,
  s2_slot_id, s2_intr_dci, s2_intr_report_len (from endpoint descriptor).
- The main poll loop at ~line 3647 is unchanged and only polls slot1.

Acceptance:
1. After slot2 Configure Endpoint ok marker, and BEFORE the
   "[sexusb.hid.{}.continuous.start]" marker for slot1:
   a. Queue one Normal TRB (IOC=1) on s2_intr_ring at index 0
      - Data buffer = s2_intr_report_phys
      - Length = s2_intr_report_len (typically 8 for usb-tablet)
      - cycle = s2_intr_pcs (initialize to 1)
   b. Ring doorbell: mmio_write32(db_base, s2_slot_id * 4, s2_intr_dci)
   c. Log: [sexusb.slot2.poll.start] slot=N dci=N
2. After the slot1 poll loop drains the event ring, slot2's TRB completion
   event MAY appear. Do NOT consume slot2 events — let them accumulate
   as unrelated events (the slot1 loop will skip them with a log line).
   This proves slot2 generated events but keeps single-device safety.
3. Build passes (./scripts/entrypoint_build.sh).
4. Single-device behavior identical when target_port_count == 1.

REQUIRED: Read docs/handoff/SEXUSB_SECOND_DEVICE_CONFIGURE_ENDPOINT_V1.md
and the slot2 resource allocation section (~lines 2596-2800 in sexusb/src/main.rs)
BEFORE writing any code.

STOP FIRST if:
- This requires changing the poll loop structure
- This requires new PDX opcodes
- First-device (slot1) behavior changes
- Build fails
- You find s2_intr_* variables are not accessible at the insertion point
  (scope issue — resolve by hoisting or storing in outer scope, not by
   restructuring the function)

OUTPUT:
1. Patch to servers/sexusb/src/main.rs
2. Updated handoff doc
3. Build confirmation
```

---

## 8. Risk List

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|------------|--------|------------|
| R1 | Event ring corruption from slot2 TRB completion during slot1 poll | Medium | High (stuck event ring) | Validate slot2 events are skipped in slot1 poll loop; do NOT consume slot2 events in C1 |
| R2 | Slot2 never produces TRB completions (QEMU zero-byte bug means no interrupt?) | Medium | Low (C1 still valuable — proves ring works) | Even zero-byte reports produce TRB completions ("actual=6, b0=0...b5=0"); the xHCI transfer still completes |
| R3 | Slot1 poll loop breaks when slot2 events appear on shared event ring | Low | High (keyboard stops) | C1 does NOT change the poll loop. Slot2 events will be "unrelated events" consumed silently |
| R4 | s2_intr_* variables not in scope at C1 insertion point | Medium | Medium | Variables are declared inside `if target_port_count > 1` block; may need to hoist to function scope |
| R5 | C2 demux starves one device while other's pdx_call blocks | Medium | High (missed input) | Keyboard re-arm-before-IPC already solves this for slot1; generalize the pattern |
| R6 | QEMU 11.0.0 never delivers non-zero pointer reports | High | High (Phase F blocked) | Acceptable — guest pipeline is proven via synthetic path. Real data requires QEMU fix or hardware boot |
| R7 | Plan touches too many subsystems at once | Low | High (scope creep) | Phases C→F are strictly sequential. Each phase gates on previous phase PASS |
| R8 | sexinput's OP_USB_MOUSE_REPORT handler removed for real USB in REAL_CLICK_TARGET_PROOF_V1 | None | None | NOT a risk — sexinput now uses OP_HID_EVENT for real path. Slot2 data flows through this proven path |
| R9 | Keyboard-only QEMU config loses slot2 | Low | Medium | When `SEXUSB_QEMU_DEVICE=kbd`, only one device exists. `target_port_count == 1`, all slot2 code skipped |
| R10 | `SingleHidBind` structure insufficient for two devices | Medium | Medium | C2 requires replacing `single_bind` with `[HidDevice; 2]` array. Scoped refactor, ~20 struct fields to duplicate |

---

## 9. Files In Scope (This Plan)

| File | Phase | Change |
|------|-------|--------|
| `servers/sexusb/src/main.rs` | C1, C2 | Add slot2 poll start (C1); convert poll loop to event demux (C2) |
| `servers/sexinput/src/main.rs` | — | No changes (normalizer already proven) |
| `servers/silk-shell/src/main.rs` | — | No changes (handlers already proven) |
| `servers/sexdisplay/src/main.rs` | — | No changes (cursor draw already proven) |
| `kernel/src/` | — | FORBIDDEN |
| `crates/sex-pdx/src/lib.rs` | — | FORBIDDEN (no ABI changes) |
| `docs/handoff/SEXUSB_INPUT_COMPLETION_PLAN_V1.md` | Plan | This document |
| `docs/handoff/SEXUSB_SLOT2_POLL_START_V1.md` | C1 | Created on implementation |
| `docs/handoff/SEXUSB_POLL_DEMUX_V1.md` | C2 | Created on implementation |

## 10. Files Explicitly Out of Scope

| File | Reason |
|------|--------|
| `kernel/src/` | No kernel/IRQ/capability edits |
| `crates/sex-pdx/src/lib.rs` | No ABI changes |
| `servers/silk-shell/src/main.rs` | Shell interaction policy unchanged |
| `servers/sexdisplay/src/main.rs` | Compositor/renderer unchanged |
| `servers/linen/` | Storage subsystem |
| `servers/sexfiles/` | Storage subsystem |
| `servers/sexbell/` | Event subsystem |
| `servers/sexdrive/` | Block I/O subsystem |

---

## 11. Build & Run Reference

### Default build (real USB path, no synthetic)
```bash
./scripts/entrypoint_build.sh
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run
```

### Multi-device config (keyboard + tablet)
```bash
# Requires both QEMU devices connected:
# -device usb-kbd,bus=xhci.0 -device usb-tablet,bus=xhci.0
./scripts/entrypoint_build.sh
SEXUSB_QEMU_DEVICE=kbd ./dev.sh run 2>&1 | tee /tmp/multidev.log
```

### Verify markers
```bash
grep -c 'sexusb.slot2.poll' /tmp/multidev.log
grep -c 'sexusb.demux' /tmp/multidev.log
grep -cE 'panic|#PF|#GP' /tmp/multidev.log
```

---

## 12. Relationship to Other Handoffs

| Handoff | Phase | Relationship |
|---------|-------|-------------|
| INPUT_PHASE_CLOSEOUT_V1 | Reference | Synthetic path fully proven |
| INPUT_REAL_DEVICE_RELIABILITY_V1 | Reference | All 6 proof markers pass |
| KEYBOARD_EDGE_PROOF_V1 | Reference | PS/2 EV_KEY path proven |
| USB_CURSOR_ROUTE_PROOF_V1 | Reference | Zero-byte QEMU reports documented |
| SEXUSB_SECOND_DEVICE_CONFIGURE_ENDPOINT_V1 | Prerequisite | Slot2 endpoint configured |
| SEXUSB_HID_MULTIDEVICE_POINTER_AUDIT_V1 | Reference | Architecture audit for multi-device |
| SEXUSB_SINGLE_DEVICE_GUARD_V1 | Reference | Guard comments at all single-device chokepoints |
| INPUT_CLICK_FOCUS_PROOF_V1 | Reference | Click-focus marker chain proven |
| REAL_CLICK_TARGET_PROOF_V1 | Reference | Ownership rules and double-apply fix |

---

*End of SEXUSB_INPUT_COMPLETION_PLAN_V1.md*
