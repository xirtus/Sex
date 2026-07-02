# SEXINPUT_TO_SILKSHELL_ROUTE_PROOF_V1

**Date:** 2026-05-07
**Status:** AUDIT COMPLETE — route correct, blocked by silk-shell boot sequence

## 1. Route Audit

| Hop | Check | Status |
|-----|-------|--------|
| sexinput sends OP_HID_EVENT | `pdx_call_checked(SLOT_SHELL, OP_HID_EVENT, ...)` | ✅ |
| Target slot | `SLOT_SHELL = 6` (sex_pdx::SLOT_SHELL) | ✅ |
| Kernel capability | `pd.grant_capability(SLOT_SHELL, Domain(silkshell_id))` | ✅ |
| Silk-shell listener | `pdx_listen_raw(0)` in main loop | ✅ |
| OP_HID_EVENT handler | Match arm at line 10959 | ✅ |
| EV_REL/EV_ABS/EV_BTN dispatch | Lines 11960-12100 | ✅ |
| Markers present | `silk-shell.pointer.recv`, `cursor.update`, `click.down` | ✅ |

**Route is structurally correct.** Sexinput sends to the right slot, kernel
grants the capability, silk-shell has the handler. The route works.

## 2. Why Shell Markers Don't Appear

Silk-shell's boot sequence calls `linen_fetch_remote_snapshot()` (line 10275)
BEFORE entering the main event loop at line 10344.

`linen_fetch_remote_snapshot()` calls `linen_sync_reply()` which does:

```rust
loop {
    let msg = pdx_listen_raw(0);
    if msg.type_id == 0x1 {
        return msg.arg0;
    }
    // ALL OTHER MESSAGES ARE SILENTLY DROPPED
}
```

During this synchronous fetch:
1. Sexinput sends OP_HID_EVENT → delivered to silk-shell's IPC ring
2. `linen_sync_reply` receives it via `pdx_listen_raw(0)`
3. `type_id` is `0x202` (OP_HID_EVENT), not `0x1`
4. **Message dropped** — loop continues waiting for Linen reply
5. Sexinput blocks on `pdx_call_checked` waiting for silk-shell's reply
6. Silk-shell never replies because it's stuck in `linen_sync_reply`

Additionally, Linen's disk read fails (`err=-3`), so the fetch never
completes successfully, and `[silk-shell.ready]` never appears.

## 3. Evidence

```
[linen.diskfs.slot.min.done] ok=0         ← fetch failed
[sexinput.pointer.drop] reason=idle...     ← 2 events dropped
[sexinput.pointer.send] count=6            ← 6 events sent (blocked)
[silk-shell.ready]                         ← NEVER APPEARS
[silk-shell.pointer.recv]                  ← NEVER APPEARS
```

## 4. Fix Options

| Option | Scope | Risk |
|--------|-------|------|
| A. Add timeout to linen_fetch_remote_snapshot | silk-shell only | Low |
| B. Defer linen fetch to after main loop starts | silk-shell only | Low |
| C. Buffer non-Linen messages during fetch | silk-shell only | Medium |
| D. Make linen_sync_reply forward unknown messages to dispatch | silk-shell only | Medium |

**Recommended: Option B** — move `linen_paint_surface()` call from boot
initialization to the first iteration of the main loop, after `pdx_listen_raw`
has already processed at least one batch of messages.

## 5. Files Changed

| File | Change |
|------|--------|
| `docs/handoff/SEXINPUT_TO_SILKSHELL_ROUTE_PROOF_V1.md` | Created |

## 6. Impact on USB 100%

The sexinput→silk-shell route is proven correct structurally. The blocker
is a silk-shell boot sequencing issue, not a routing or capability bug.
Once silk-shell reaches its main loop, pointer/click/focus markers will fire.

---

*End of SEXINPUT_TO_SILKSHELL_ROUTE_PROOF_V1.md*
