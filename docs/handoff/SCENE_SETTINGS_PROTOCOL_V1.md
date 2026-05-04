# SCENE_SETTINGS_PROTOCOL_V1

## Status

Complete (2026-05-04). Local `OP_SCENE_SETTINGS_CMD = 0xFB` handler implemented
in silk-shell's main loop. All 8 command IDs supported. Build passes:
`[SEXOS ENTRYPOINT] success`.

---

## Changed Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Added `OP_SCENE_SETTINGS_CMD = 0xFB` const; added `CMD_*` constants (1-8); added `handle_scene_settings_cmd()` helper; added dispatch arm in main loop's `type_id` match before `0x1` (sexstore reply); added `[shell.scene.settings.cmd]` markers (budget 32) |
| `docs/handoff/SCENE_SETTINGS_PROTOCOL_V1.md` | New — this document |

### NOT modified

- `crates/sex-pdx/src/lib.rs` — Opcode kept local (Option B per plan). No ABI hash change.
- `sexos_build_spec.toml` — No ABI_VERSION bump.
- `servers/sexdisplay/` — No renderer changes.
- `kernel/` — FORBIDDEN.
- `servers/sexinput/` — Proof deferred.
- `servers/sexstore/` — Persistence path unchanged.
- `servers/silkbar/` — Unrelated.
- `servers/sexusb/` — Unrelated.
- Any other file.

---

## Local Opcode / Command Table

```rust
const OP_SCENE_SETTINGS_CMD: u64 = 0xFB;

const CMD_SET_PRESET: u64 = 1;
const CMD_CYCLE_PRESET: u64 = 2;
const CMD_SET_TINT: u64 = 3;
const CMD_CYCLE_TINT: u64 = 4;
const CMD_SET_CHROME_FLAGS: u64 = 5;
const CMD_TOGGLE_TOP_BAR: u64 = 6;
const CMD_SET_ACCESSIBILITY: u64 = 7;
const CMD_RESET_DEFAULTS: u64 = 8;
```

### Command dispatch

```
pdx_call(SLOT_SHELL, 0xFB, cmd_id, value, flags)
       -> silk-shell main loop detects type_id == 0xFB
       -> handle_scene_settings_cmd(cmd_id, value, flags)
       -> pdx_reply(0)
       -> mutated = true
```

---

## Mutation Rules

| Command | Mutates | Persists | Reuses |
|---------|---------|----------|--------|
| `CMD_SET_PRESET` (1) | `preset_idx = val % PRESET_COUNT`; clears `use_custom_colors`, `custom_colors`, `ACTIVE_TINT_IDX` | Yes sexstore PUT | `resolve_scene_render_tokens()` + `push_token_preset()` |
| `CMD_CYCLE_PRESET` (2) | Same as F5 — cycles forward, clears custom/tint | Yes sexstore PUT | `cycle_scene_render_token_preset()` (full existing helper) |
| `CMD_SET_TINT` (3) | `ACTIVE_TINT_IDX = val % TINT_COUNT`; applies tint bundle | No Ephemeral | `apply_custom_tint_bundle()`, `resolve_scene_render_tokens()` |
| `CMD_CYCLE_TINT` (4) | Same as F6 — cycles tint, ephemeral | No Ephemeral | `cycle_custom_tint()` (full existing helper) |
| `CMD_SET_CHROME_FLAGS` (5) | `chrome_flags = val as u8` | Yes sexstore PUT | `resolve_scene_render_tokens()` + `push_token_preset()` |
| `CMD_TOGGLE_TOP_BAR` (6) | Calls `toggle_top_bar_for_active_frame()` | No Frame flag | Existing helper |
| `CMD_SET_ACCESSIBILITY` (7) | `accessibility_flags = val as u8` | Yes sexstore PUT | `resolve_scene_render_tokens()` + `push_token_preset()` |
| `CMD_RESET_DEFAULTS` (8) | `SCENE_APPEARANCE_STATE = DEFAULT_SCENE_APPEARANCE`; `ACTIVE_TINT_IDX = 0` | Yes sexstore PUT | `resolve_scene_render_tokens()` + `push_token_preset()` |
| Unknown (any other) | None — logged and ignored | No | N/A |

### Persistence detail

Persisting commands pack a blob via `pack_scene_settings_blob()` containing:
- `preset_idx`, `chrome_flags`, `accessibility_flags`
- Fire `pdx_call(SLOT_SEXSTORE, OP_KV_PUT, 0x01, blob, 0)` — fire-and-forget

`CMD_SET_PRESET` and `CMD_RESET_DEFAULTS` construct the blob inline.
`CMD_CYCLE_PRESET` delegates to `cycle_scene_render_token_preset()` which already persists.

All commands that call `cycle_scene_render_token_preset()` or `cycle_custom_tint()`
inherit their existing marker behavior (preset cycle marker, tint cycle marker, save marker).
The protocol handler adds its own `[shell.scene.settings.cmd]` marker on top.

---

## Handler Structure

```rust
unsafe fn handle_scene_settings_cmd(cmd: u64, value: u64, _flags: u64) {
    static mut CMD_BUDGET: u32 = 32;
    let b = &mut CMD_BUDGET;
    match cmd {
        CMD_SET_PRESET       => { /* clamp idx, mutate, push, persist, marker */ }
        CMD_CYCLE_PRESET     => { /* cycle_scene_render_token_preset(), marker */ }
        CMD_SET_TINT         => { /* clamp idx, apply, push, marker (no persist) */ }
        CMD_CYCLE_TINT       => { /* cycle_custom_tint(), marker */ }
        CMD_SET_CHROME_FLAGS => { /* update flag, push, persist, marker */ }
        CMD_TOGGLE_TOP_BAR   => { /* toggle_top_bar_for_active_frame(), marker */ }
        CMD_SET_ACCESSIBILITY=> { /* update flag, push, persist, marker */ }
        CMD_RESET_DEFAULTS   => { /* reset state, push, persist, marker */ }
        _                    => { /* marker with ok=0 unknown */ }
    }
}
```

### Main loop dispatch (inserted before `0x1` reply arm)

```rust
OP_SCENE_SETTINGS_CMD => {
    unsafe {
        handle_scene_settings_cmd(msg.arg0, msg.arg1, msg.arg2);
    }
    pdx_reply(0);
    mutated = true;
}
```

Never blocks. Invalid cmd/value silently clamped or ignored.

---

## Markers

| Marker | When | Budget |
|--------|------|--------|
| `[shell.scene.settings.cmd] cmd=N preset=N ok=1` | CMD_SET_PRESET succeeded | 32 |
| `[shell.scene.settings.cmd] cmd=2 ok=1` | CMD_CYCLE_PRESET succeeded | 32 |
| `[shell.scene.settings.cmd] cmd=3 tint=N ok=1` | CMD_SET_TINT succeeded | 32 |
| `[shell.scene.settings.cmd] cmd=4 ok=1` | CMD_CYCLE_TINT succeeded | 32 |
| `[shell.scene.settings.cmd] cmd=5 flags=N ok=1` | CMD_SET_CHROME_FLAGS succeeded | 32 |
| `[shell.scene.settings.cmd] cmd=6 ok=1` | CMD_TOGGLE_TOP_BAR succeeded | 32 |
| `[shell.scene.settings.cmd] cmd=7 flags=N ok=1` | CMD_SET_ACCESSIBILITY succeeded | 32 |
| `[shell.scene.settings.cmd] cmd=8 ok=1` | CMD_RESET_DEFAULTS succeeded | 32 |
| `[shell.scene.settings.cmd] cmd=N ok=0 unknown` | Unknown command ID received | 32 |

All markers share the same budget of 32 (`static mut CMD_BUDGET: u32 = 32`).

### Example marker output

```
[shell.scene.settings.cmd] cmd=1 preset=0 ok=1
[shell.scene.settings.cmd] cmd=8 ok=1
[shell.scene.settings.cmd] cmd=99 ok=0 unknown
```

---

## Safety

- All `unsafe` blocks use the same pattern as existing code (`static mut` for budget variables).
- `SCENE_APPEARANCE_STATE` is a `static mut` — the handler runs in the main loop's
  single-threaded context, same as all other `unsafe fn` that mutate this state.
- Input clamping:
  - `CMD_SET_PRESET`: `value` clamped to `value < PRESET_COUNT` else 0
  - `CMD_SET_TINT`: `value % TINT_COUNT` — wraps safely
  - `CMD_SET_CHROME_FLAGS`, `CMD_SET_ACCESSIBILITY`: truncated to `u8`
- Return value: `pdx_reply(0)` always. No error codes in V1.
- `_flags` (arg2) is reserved and ignored; no interpretation in V1.

---

## Panel Key Preservation

Panel keys (F4/F5/F6/F7, Esc/1/2/3 when panel visible) are unchanged.
The new `OP_SCENE_SETTINGS_CMD` handler is a **separate dispatch path** in the
`type_id` match — it responds to PDX calls from other PDs, not to keyboard input.

The existing panel key intercept (pre-dispatch scancode routing) and normal
`SurfaceAction` dispatch are untouched.

---

## Build Result

```
[SEXOS ENTRYPOINT] success
```

No ABI hash update required (sex-pdx unchanged). No new warnings.
Only files changed: `servers/silk-shell/src/main.rs` + this handoff doc.

---

## Limitations

| Limitation | Impact |
|------------|--------|
| **Local opcode** | No external PD (Settings app) can use `OP_SCENE_SETTINGS_CMD` as a named constant until promoted to `sex-pdx`. Works via raw `0xFB`. |
| **No Settings app PD** | Protocol is defined but unexercised by real callers. Only testable via synthetic proof or future app. |
| **No caller policy** | Any PD that can reach `SLOT_SHELL` can send any command. Collar authentication deferred. |
| **No error codes** | All replies are `pdx_reply(0)`. No way for caller to distinguish "applied" from "clamped/ignored". |
| **No rate limiting** | Handler is synchronous and bounded, but no per-caller throttling. |
| **Chrome flags no-op on renderer** | `chrome_flags` bit 0 reserved for top-bar — not interpreted by sexdisplay in V1. |
| **Accessibility flags no-op** | Bit 0=high_contrast, bit 1=colorblind_safe — reserved; no behavior implemented. |

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_SETTINGS_PROTOCOL_PLAN_V1.md` | Design plan with Option B (local opcode) rationale |
| `docs/handoff/SCENE_SETTINGS_PANEL_KEYS_V1.md` | Panel key intercept (Esc/1/2/3) — preserved unchanged |
| `docs/handoff/SCENE_SETTINGS_PERSIST_V1.md` | Persistence path reused for SET_PRESET/SET_CHROME/SET_ACCESSIBILITY/RESET |
| `docs/handoff/SCENE_SETTINGS_INMEM_V1.md` | SceneAppearanceState, resolve_scene_render_tokens |
| `docs/handoff/SCENE_RENDER_TOKEN_PRESETS_V1.md` | TOKEN_PRESETS, cycle_scene_render_token_preset |
| `docs/handoff/SCENE_CUSTOM_COLOR_KEYS_V1.md` | CUSTOM_TINT_BUNDLES, ACTIVE_TINT_IDX, cycle_custom_tint |
| `servers/silk-shell/src/main.rs` | Implementation |

---

## Next Recommended Phase

**SCENE_SETTINGS_PROTOCOL_SYNTH_PROOF_V1** — Add synthetic keyboard proof in
`servers/sexinput/src/main.rs` that exercises `OP_SCENE_SETTINGS_CMD` with
each command ID, verifies markers in serial output, and confirms persistence
fires for persistable commands.

Scope:
- Add `SEXOS_SETTINGS_PROOF` feature gate (follows existing `option_env!` pattern)
- For each command ID: send `pdx_call(SLOT_SHELL, 0xFB, cmd_id, value, 0)`
- Verify: `[shell.scene.settings.cmd]` marker appears
- Verify: `[sexstore.kv.put]` marker appears for persistable commands (1, 2, 5, 7, 8)
- Verify: `[shell.appearance.preset]` or `[shell.appearance.custom]` markers for state changes
- Zero faults
