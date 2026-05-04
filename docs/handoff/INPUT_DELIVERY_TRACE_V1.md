# INPUT_DELIVERY_TRACE_V1

**Date:** 2026-05-03
**Status:** DIAGNOSED

## Symptoms

1. Cursor blinked, moved once/twice to ~x=500 y=600, then stopped
2. Boxes/panels/windows opened and closed behind/around the cursor
3. `usb-tablet` QEMU device did not fix usability
4. No panic/PF/GP

## Diagnosis: `SEXOS_PROOFS_DISABLED=1` set at RUNTIME, not BUILD TIME

**Root cause:** `SEXOS_PROOFS_DISABLED` is read via Rust's `option_env!("SEXOS_PROOFS_DISABLED")`
which is evaluated at **compile time**. Setting it in `dev.sh run` (or any QEMU invocation)
has zero effect. The ISO binary was built with the default `option_env!` returning `None`,
so `SYNTHETIC_INPUT_PROOFS_DISABLED = false` — all proofs active.

### Proof gate mechanism

| Component | Has gate? | Mechanism |
|-----------|-----------|-----------|
| sexinput  | YES       | `const SYNTHETIC_INPUT_PROOFS_DISABLED: bool = option_env!("SEXOS_PROOFS_DISABLED").is_some();` |
| silk-shell| NO        | No synthetic/demo code at all |
| sexusb     | NO        | Forwards reports unconditionally |
| sexdisplay | NO        | Renders unconditionally |

### What fires when proofs are active

| Proof source | sexinput lines | Effect |
|-------------|----------------|--------|
| Drag proof | 216-243 | Moves cursor, holds button, drags ~6,4, releases (every 120 ticks, 3 cycles, then done) |
| SilkBar click proof | 249-370 | Clicks launcher, workspace, status, clock, bell, then closes them |
| Click-focus proof | 377-403 | Sets cursor to (940,560), clicks linen surface |

### Diagnostic trace comparison

**Build WITHOUT env var, run with `SEXOS_PROOFS_DISABLED=1` at runtime (broken):**
```
[proof.gate.state] enabled=1 source=default
[sexinput.synthetic.silkbar_click] target=launcher
[sexinput.synthetic.silkbar_click] target=workspace index=3
[sexinput.synthetic.click_focus.start]
...
[sexinput.drag_proof.start]
...
```
→ 16 synthetic proof markers. Panels open/close. Cursor jumps.

**Build WITH `SEXOS_PROOFS_DISABLED=1`, run without env var (correct):**
```
[proof.gate.state] enabled=0 source=env
```
→ 0 synthetic proof markers. Clean boot. Tablet idle.

## Bounded diagnostic markers added

| Marker | File | Budget | Purpose |
|--------|------|--------|---------|
| `[proof.gate.state]` | sexinput/src/main.rs | 1 | Shows enabled/disabled at boot |
| `[sexusb.forward.mouse]` | sexusb/src/main.rs | 16 | Shows forwarded buttons + packed axes to sexinput |

(Existing markers `[sexinput.mouse.real.delta]`, `[sexinput.mouse.real.button]`,
`[shell.cursor.move]`, `[shell.click.real.target]` were already present from prior
diagnostic rounds.)

## Key traces

### Tablet idle reports (nographic, no user input)
```
[sexusb.hid.tablet.raw] b0=0x0 b1=0x0 b2=0x0 b3=0x0 b4=0x0 actual=6
[sexusb.forward.mouse] buttons=0x0 packed=0x0
[sexinput.mouse.real.delta] dx=0 dy=0 buttons=0x0
```
→ Tablet reports reach sexusb, decode correctly, forward to sexinput.
→ sexinput normalizes (zero data = no EV_REL/EV_BTN forwarded).
→ Clean idle loop, no synthetic events when gate is active.

## Fix

**No code change needed.** The fix is the correct build workflow:

```sh
# Interactive visual mode — proofs disabled
SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
SEXUSB_QEMU_DEVICE=tablet ./dev.sh run
```

The env var must be set during `./scripts/entrypoint_build.sh`, not in `dev.sh run`.

## Prevention

Added "Critical Workflow Note" to `INTERACTIVE_MODE_PROOF_GATE_V1.md` documenting
this distinction. The `[proof.gate.state]` marker provides immediate boot-time
confirmation.

## STOP conditions met

- [x] env/proof gating mechanism is now clear (compile-time option_env!)
- [x] QEMU device config is clear (SEXUSB_QEMU_DEVICE=tablet selects usb-tablet)
- [x] No fix requiring new ABI or renderer redesign
- [x] No kernel/PDX/display changes
- [x] No panic/PF/GP observed
