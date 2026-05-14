# USB_SLOT2_MOUSE_BLOCKED_HANDOFF_V1

Date: 2026-05-14
Mission: USB_SLOT2_MOUSE_BLOCKED_HANDOFF_V1
Scope: docs-only blocker handoff for slot2 USB mouse in multi-HID lane

## 1. Current Status
- `usb-kbd + usb-mouse` lane: keyboard works, mouse as slot2 does not produce continuous motion.
- Mouse-only slot1 lane previously works.

## 2. Proven
- Slot2 mouse endpoint descriptor: `addr=0x81 dci=3 mps=4 interval=7`.
- Slot2 EP context coherent: `add_flags=0x9`, `type=7` interrupt-IN, `cerr=3`, dequeue matches ring.
- Slot2 slot context coherent: `route=0`, `speed=3`, `ctx_entries=3`, `root_hub_port=6`.
- Configure Endpoint command completes with `cc=1`.
- TRB write physical address matches ring physical address.
- Doorbell `slot=2 dci=3` fires.

## 3. Known-Good Comparison
- Slot1 keyboard shows same `type/dci/interval/cerr/context_entries` profile.
- Only expected differences are `mps/max_esit/avg_len` and `root_hub_port`.

## 4. Failing Evidence
- Event ring shows repeated `slot=1 ep_id=3` transfer events only.
- `saw_slot2` remains `0`.
- No `slot2.mouse.report/forward/requeue` activity.

## 5. Suspicion
- xHCI endpoint running/startup issue for second HID interrupt endpoint, not downstream IPC.
- Possible missing multi-slot scheduling nuance, configure sequencing, endpoint state transition, or QEMU xHCI behavior.

## 6. Recommendation
- Do not merge diagnostic patches until a minimal fix is proven.
- Use slot1 mouse-only lane for pointer regression.
- Use keyboard-first and synthetic proofs for GUI work.
- Revisit slot2 in a dedicated USB multi-HID milestone.
