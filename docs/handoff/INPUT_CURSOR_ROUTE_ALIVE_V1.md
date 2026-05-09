# INPUT_CURSOR_ROUTE_ALIVE_V1

**Date:** 2026-05-08
**Status:** ROUTE PROVEN — CURSOR MOVEMENT ALIVE AT RUNTIME

---

## A. Route Map

```
sexusb → sexinput → silk-shell → sexdisplay
```

### Hop detail

| Hop | From | To | Opcode | Slot | Payload |
|-----|------|----|--------|------|---------|
| 1 | sexusb | sexinput | `OP_USB_MOUSE_REPORT` (0x260) | 9 | raw HID report |
| 2 | sexinput | silk-shell | `OP_HID_EVENT` (0x202) | 6 | class=2 (EV_REL), dx, dy |
| 3 | silk-shell | sexdisplay | `OP_SURFACE_UPDATE` (0x100) | 5 | SURFACE_ID_CURSOR, x, y |

### Critical path detail

silk-shell `linen_sync_reply()` — the blocking helper that waits for Linen paint replies —
formerly consumed and acked non-reply `OP_HID_EVENT` messages.  Pointer events arriving
during Linen composition were silently dropped before HID dispatch, making the cursor
appear frozen.

Fix: `linen_sync_reply()` now detects `OP_HID_EVENT`, calls `apply_rel_pointer()` inline,
and continues waiting for Linen replies.  A pre-Linen non-blocking input drain is also
present.

### Three EV_REL dispatch sites (consistent behaviour)

1. Main match block (normal event loop)
2. `linen_sync_reply()` (during Linen paint — was the broken site)
3. Pre-Linen non-blocking drain (before Linen paint starts)

All three call the same `apply_rel_pointer()` helper.

---

## B. Proof Grep

### Canonical chain (real USB mouse, QEMU or hardware)

```bash
# sexusb → sexinput: raw HID payload arriving
rg 'sexinput.pointer.raw' /tmp/sexos.log

# sexinput → silk-shell: EV_REL emission
rg 'sexinput.pointer.send.*class=2' /tmp/sexos.log
rg 'sexinput.hid.emit.rel' /tmp/sexos.log

# silk-shell: receipt during Linen sync (the previously-broken site)
rg 'silk-shell.linen_sync.input_hid.*class=2' /tmp/sexos.log

# silk-shell: cursor surface update to sexdisplay
rg 'sexdisplay.cursor.surface.update.*n=0' /tmp/sexos.log

# sexdisplay: cursor draw with non-center coordinates
rg 'sexdisplay.cursor.draw.*n=0' /tmp/sexos.log
```

### Fault check

```bash
rg -c 'fault.kill|#PF|#GP|panic|FATAL' /tmp/sexos.log
# Must be 0
```

### Budgeted pointer filter proof

```bash
rg 'silk-shell.pointer.filter' /tmp/sexos.log
# Shows raw vs filtered dx/dy when gain reduction is active (budget 32)
```

---

## C. Known Failure Class

**Blocking reply helpers that ack/drop unrecognised opcodes.**

Pattern: a function loops on `pdx_try_listen_raw()` or `pdx_listen_raw()` waiting for a
specific reply opcode, and silently acks everything else.  If `OP_HID_EVENT` arrives
during the wait, it is consumed and lost — cursor input dies during that blocking phase.

Remediation: every such loop must handle `OP_HID_EVENT` inline and continue waiting.

Audited sites (2026-05-08):
- `linen_sync_reply()` — FIXED
- All other reply-helper loops in silk-shell — none exhibit this class (they wait on
  specific known opcodes and defer unknown opcodes to main loop)

---

## D. No-Touch Boundaries

**STOP FIRST** before modifying any of these for pointer quality work:

| Boundary | Reason |
|----------|--------|
| Kernel (`kernel/src/`) | IPC path; regressions fatal |
| ABI (`crates/sex-pdx/`, `crates/sex-abi/`) | Opcode/slot definitions; all PDs must agree |
| sexdisplay (`servers/sexdisplay/`) | Cursor renderer; display pipeline is fragile |
| Build scripts (`scripts/`, `crates/build/`) | No new env vars for pointer tuning without gate |

Pointer quality tuning (gain, clamping, smoothing) MUST stay inside `servers/silk-shell/`
and `servers/sexinput/` only.  No kernel, ABI, or sexdisplay changes for smoothness.

---

## E. Next Prompt

```
POINTER_QUALITY_V1
```

Remaining work:
- Gain reduction tuning (current: `/2`, clamped to ±32)
- Acceleration profile (none yet — linear only)
- Budgeted filter markers are in place (`[silk-shell.pointer.filter]` budget 32)
- Clamp markers are in place (`[silk-shell.cursor.clamp]` budget 16)
- Raw payload markers are in sexinput (`[sexinput.pointer.raw]` budget 64)
- Real hardware tablet absolute-coordinate path (separate from EV_REL)
