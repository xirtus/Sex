# INTERACTIVE_MODE_PROOF_GATE_V1

**Date:** 2026-05-03
**Status:** MERGED

## Symptom

Synthetic input proofs (drag, click-focus, silkbar clicks) run during early boot
and fight the user during interactive visual use. Specific observed issues:

- **Cursor jumping** — proofs move `POINTER_X/Y` and click at automated positions
  while the user is trying to control the mouse
- **Panels opening/closing** — the silkbar click proof opens and closes launcher,
  status, clock, and bell panels automatically at boot
- **No way to disable** — all three proof gates were hardcoded `const false`
  (proofs always on), requiring source edits to toggle

## Fix

Added a single compile-time env-var gate:

```rust
const SYNTHETIC_INPUT_PROOFS_DISABLED: bool =
    option_env!("SEXOS_PROOFS_DISABLED").is_some();
```

- **Default (env var unset):** proofs enabled — CI/nographic verification works
  as before
- **Set `SEXOS_PROOFS_DISABLED=1` at build time:** all three synthetic proof
  blocks are skipped entirely

The three previous `USB_PROOF_DISABLE_*` constants are replaced by this single
gate. No proof code was removed — it's all behind `if !SYNTHETIC_INPUT_PROOFS_DISABLED`.

## Build Commands

```sh
# Proof mode (CI / nographic verification) — proofs enabled, default
./scripts/entrypoint_build.sh
SEXUSB_XHCI_TRACE=0 timeout 15 ./dev.sh run-nographic

# Interactive visual mode — proofs disabled, clean boot
SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
SEXUSB_XHCI_TRACE=0 SDL_VIDEO_DRIVER=x11 ./dev.sh run
```

## Changed Files

- `servers/sexinput/src/main.rs` — replaced three `const USB_PROOF_DISABLE_*`
  with one `const SYNTHETIC_INPUT_PROOFS_DISABLED` using `option_env!`

## Verification

### Proof mode (default, nographic)

```
sexinput.drag_proof.done:  1
shell.drag.start:          1
shell.drag.move:           1
shell.drag.end:            1
shell.click.real.target:  10
panic/PF/GP:               0
```

### Interactive mode (SEXOS_PROOFS_DISABLED=1)

Zero synthetic proof markers. Boot is clean — no automated cursor movement,
no panel toggles, no focus changes.

### Visual check

```sh
SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
SEXUSB_XHCI_TRACE=0 SDL_VIDEO_DRIVER=x11 ./dev.sh run
```

Confirm:
- No automated cursor jumping
- No weird window/panel opening
- Clock counts
- Real mouse moves cursor
- App click focuses app
- Hold-left on app + move drags floating surface
- Release ends drag
- SilkBar clicks do not start drag

## Changed Invariants

1. Synthetic proofs are gated by a single compile-time env var, not three consts
2. Default build (no env var) preserves full proof capability for CI
3. Interactive mode requires explicit `SEXOS_PROOFS_DISABLED=1` to suppress proofs
4. No proof code removed — all blocks conditionally compiled at runtime via const
5. No kernel/PDX/ABI changes — pure Rust compile-time env embedding

## STOP FIRST Conditions

1. Adding runtime env detection in no_std without kernel support
2. Removing proof code instead of gating it
3. Changing the default (proofs enabled) — would break CI verification
4. Adding IPC/channels for runtime proof toggle — unnecessary complexity
