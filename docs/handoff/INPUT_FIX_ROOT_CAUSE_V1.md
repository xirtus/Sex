# INPUT_FIX_ROOT_CAUSE_V1 — Handoff

Date: 2026-07-02
Baseline: branch `master` @ `da5dd87a`, dirty tree (docs mass-deleted, dev.sh diag lines).
Build: `scripts/entrypoint_build.sh` PASS (before and after patch).

## Verdict: FAIL (current tier) — honestly, with root causes identified

`scripts/input_current_tier_gate.sh <serial-log>` now exists and fails until every
current-tier marker is present. Fault scan on all four proof runs: **zero** #PF/#GP/
panic/fault.kill/storm.

## What is PROVEN with real QEMU USB events (QMP injection, headless)

Chain: usb-tablet → sexusb xHCI interrupt-IN → sexinput normalize → OP_HID_EVENT →
silk-shell → cursor/focus:

- pointer movement: `[usb.pointer.shell.apply]`, `[input.pointer.move.ok]`
- button down/up edges: `[input.button.down.ok]` / `[input.button.up.ok]`
- click → live hit → focus: `[silk.click.hit.live.ok]`, `[silk.focus.set.ok]`
  (real click observed: focus.set id=202 at real cursor coords)
- sustained runtime 150s, zero faults: `[input.faultscan.ok]`

Contrary to `USB_STATUS.md` (2026-05-03): **QMP `input-send-event` DOES reach
usb-tablet** (abs + buttons) in QEMU 11.x headless. That doc's claim is stale for
the tablet. It remains true for the keyboard (below).

## Root causes found (first broken links)

### 1. Pointer unpredictability: sexusb single-outstanding-TRB + slot2 starvation
`servers/sexusb/src/main.rs` main HID loop keeps ONE interrupt-IN TRB in flight for
the primary device (single_bind = slot1) and re-arms slot2 (tablet, in kbd+tablet
config) only inside the slot-mismatch demux branch. The wait loop `break`s after
servicing ONE event, then re-submits a slot1 TRB even when the event was slot2's.
Re-arm latency (cooperative sys_yield scheduling + heavy serial logging) means QEMU's
16-deep HID queue coalesces/drops most reports.
**Measured delivery: run1 11/21 moves, run2 2/28, run4 1/12.** This variance is
exactly the "unpredictable, barely works" experience of the last 3 months.
Fix = keep N (e.g. 8) TRBs queued per interrupt endpoint ring so xHCI completes
back-to-back without waiting for software re-arm. **STOP FIRST (sexusb edit).**

### 2. Keyboard headless: host-side QEMU routing, NOT guest code
With `-display none`, untargeted QMP key events go to QEMU's text console, never to
usb-kbd (32/32 reports all-zero even with `i8042=off`). Device-targeted injection
(`"device":"usbkbd"`) **crashes QEMU 11.0.1** (abort in object_property_find_err,
"Property 'qemu-fixed-text-console.device' not found") — QEMU bug.
Guest usb-kbd decode path was previously proven interactive (SDL). Environmental
blocker for headless automation; interactive SDL run required for keyboard markers.

### 3. Kernel PS/2 IRQ1 dead after boot (why PS/2 fallback never worked)
`kernel/src/interrupts.rs` keyboard_interrupt_handler: when `read_scancode()` returns
None (keyboard not yet initialized), the handler `return`s **without send_eoi()**.
A spurious boot-time IRQ1 (observed: `[ps2.irq1.entry] n=1` during init, no port60
read, no eoi) leaves vector 0x21 in-service → IRQ1 blocked forever → zero PS/2 key
delivery all session. Matches the old gate note "QEMU sendkey no PS/2 IRQ1 delivery
(environmental limitation)" — it was NOT environmental. **STOP FIRST (kernel edit):
add send_eoi() before the early return.**

### 4. Why previous proofs didn't catch any of this
Synthetic proofs (fixed coords 624,263) drive the same handle_hid_event code and
emitted look-alike success markers every boot. No gate required the real-USB chain
markers and behavior markers together.

## Patch applied (smallest safe, two ownership domains)

- `servers/sexinput/src/main.rs`: `[input.keyboard.keydown.ok]` / `[input.keyboard.keyup.ok]`
  at successful send_shell_hid_event sites (budget 8).
- `servers/silk-shell/src/main.rs`: `[input.pointer.move.ok]`, `[input.button.down.ok]`,
  `[input.button.up.ok]`, `[silk.click.hit.live.ok]`, `[silk.focus.set.ok]`,
  `[silk.drag.begin.ok]` (content + chrome sites), `[silk.drag.move.ok]`,
  `[silk.drag.end.ok]` — all at existing real behavior sites, budget-bounded.
- `scripts/input_current_tier_gate.sh` (new): requires ALL markers + usb.* upstream
  chain (rejects synthetic-only) + fault scan. Exits nonzero on FAIL.

Caveat: click/drag markers also fire on the boot synthetic proof (same code path).
The gate's usb.* requirements prevent a synthetic-only PASS of the full gate, but a
per-marker synthetic/real source tag is future work.

## Current gate result (run4, real QMP injection)

PASS: usb_pointer_emit, usb_pointer_shell_apply, pointer_move, button_down, button_up,
click_hit_live, focus_set, drag_begin, drag_end, faultscan.
FAIL: keyboard_keydown, keyboard_keyup (host routing blocker), drag_move (real drag
never accumulated moves — event loss, root cause #1).
**INPUT_100_CURRENT_TIER_V1: FAIL** (correct and honest).

## Repro commands

```bash
# boot headless with QMP
SEXOS_QEMU_DISPLAY=none SEXUSB_QEMU_DEVICE=kbd+tablet SEXOS_QEMU_I8042=off \
SEXOS_QEMU_QMP=1 SEXOS_QMP_SOCK=/tmp/sexos-qmp.sock ./dev.sh run-nographic > /tmp/serial.log 2>&1 &
# wait for [usb.xhci.enum.done], then inject abs/btn/key via QMP input-send-event
# (do NOT use "device" targeting — crashes QEMU 11.0.1)
scripts/input_current_tier_gate.sh /tmp/serial.log
```

## STOP FIRST queue (in priority order)

1. sexusb multi-TRB interrupt-IN queue (root cause #1) — unblocks drag_move and all
   pointer reliability. Bounded change: pre-queue N TRBs per endpoint ring, re-arm
   one per completion. NOT a USB rewrite.
2. kernel IRQ1 missing-EOI one-liner (root cause #3) — revives PS/2 fallback tier.
3. Marker source honesty tag (synthetic=0/1 field on click/drag markers).

## Next smallest prompt

MISSION: INPUT_POINTER_CADENCE_FIX_V1 — in servers/sexusb/src/main.rs ONLY, keep 8
interrupt-IN TRBs outstanding per HID endpoint ring (slot1 and slot2), re-arm one per
Transfer Event, no other changes. Proof: ≥90% of 20 QMP abs moves produce
[usb.pointer.shell.apply], then [silk.drag.move.ok] fires for a real drag, then
scripts/input_current_tier_gate.sh moves drag_move to PASS. STOP FIRST triggered and
approved for this bounded sexusb edit.
