# INPUT_ROUTE_PROOF_AND_MINFIX_V1

## Problem

Input route was low-confidence. Synthetic input proofs were hardcoded OFF
(`SYNTHETIC_INPUT_PROOFS_DISABLED = true; // FORCED OFF`), preventing runtime
verification of the sexinput → silk-shell → sexdisplay pointer pipeline without
QEMU QMP injection.

## Root Cause

`servers/sexinput/src/main.rs` line 36:
```rust
const SYNTHETIC_INPUT_PROOFS_DISABLED: bool = true; // FORCED OFF
```

Despite the doc comment above it (lines 19–34) describing the env-var mechanism:
```rust
/// Set env var `SEXOS_PROOFS_DISABLED=1` at build time to disable all proofs
/// for interactive visual use. Default (unset): proofs enabled for CI/nographic
/// verification.
```

The constant was hardcoded to `true`, ignoring the `option_env!("SEXOS_PROOFS_DISABLED")`
pattern used elsewhere (e.g., `KEYBOARD_CURSOR_ENABLED` on line 41).

## First Dead Hop

**sexinput → silk-shell**: sexinput never generated synthetic events, and no USB HID
data arrives in headless mode without QMP injection. The pipeline from synthetic
source to cursor draw was entirely idle.

## Fix

**Minimal 1-line change** in `servers/sexinput/src/main.rs`:

```diff
- const SYNTHETIC_INPUT_PROOFS_DISABLED: bool = true; // FORCED OFF
+ const SYNTHETIC_INPUT_PROOFS_DISABLED: bool = option_env!("SEXOS_PROOFS_DISABLED").is_some();
```

Default behavior (no env var): synthetic proofs ENABLED.
`SEXOS_PROOFS_DISABLED=1` at build time: proofs DISABLED for interactive use.

## Files Changed

| File | Change |
|------|--------|
| `servers/sexinput/src/main.rs` | 1 line: restore env-var mechanism for `SYNTHETIC_INPUT_PROOFS_DISABLED` |
| `docs/handoff/INPUT_ROUTE_PROOF_AND_MINFIX_V1.md` | NEW — this handoff |

## Build Result

**PASS** — ISO builds cleanly with no errors.

## Runtime Marker Chain (PASS)

| Hop | Marker | Evidence |
|-----|--------|----------|
| 1. Synthetic source enabled | `[proof.gate.state] enabled=1 source=default` | Confirms proofs active |
| 2. Synthetic event generation | `[sexinput.synthetic.silkbar_click] target=launcher` | sexinput emits EV_ABS at (100,25) |
| 3. silk-shell receives | `[silk-shell.pointer.recv] class=3 a0=100 a1=25` | silk-shell dispatches OP_HID_EVENT |
| 4. silk-shell updates cursor | `[silk-shell.cursor.update] x=100 y=25` | silk-shell sends OP_SURFACE_UPDATE to sexdisplay |
| 5. sexdisplay draws cursor | `[sexdisplay.cursor.draw] n=0 x=100 y=25` | sexdisplay renders at target position |

Cursor moves from default (640,360) to (100,25), (635,25), (940,25), etc.,
confirming the full pipeline delivers pointer data end-to-end.

## Additional Proved Sub-paths

- **EV_REL path**: `[sexinput.drag_proof.down]` → `[silk-shell.pointer.recv] class=2 a0=6 a1=4` (EV_REL) → `[silk-shell.cursor.update] x=206 y=204`
- **EV_BTN path**: `[sexinput.synthetic.click_focus.down/.up]` → button edges processed by silk-shell
- **SilkBar click path**: `[sexinput.synthetic.silkbar_click]` → multiple panel coordinates hit

## Safety

- **Faults**: 0 (no panic, #PF, #GP, triple fault, fault.kill, FATAL)
- **PDs spawned**: 7 (all present, all round-robinned)
- **Clock**: 12+ silkbar clock ticks
- **Keyboard controls**: unchanged (keyboard path is separate from synthetic proofs)

## Recurring Issue Saved

The `// FORCED OFF` pattern is a maintenance trap. The env-var mechanism was
already documented in the comments but the constant body contradicted it.
Future agents: prefer `option_env!("VAR").is_some()` pattern over hardcoded
constants when a build-time toggle is intended.

## Test Commands

```sh
# Build with proofs enabled (default, CI/nographic mode):
./scripts/entrypoint_build.sh

# Build with proofs disabled (interactive visual mode):
SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh

# Verify the chain (boot headless, check serial):
qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci -device usb-tablet,bus=xhci.0 \
  -serial stdio -display none
```

## Regression Risk

The synthetic drag proof is bounded (3 stages, one-shot via `SYNTHETIC_DRAG_PROOF_DONE`).
The silkbar click proof runs a fixed sequence. Both use the standard `OP_HID_EVENT`
path — same as real USB input. No shell policy changes. No kernel/ABI changes.
