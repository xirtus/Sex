# SCENE_SETTINGS_PROTOCOL_PLAN_V1

## Status

Design (2026-05-04). Protocol design for future Settings app → silk-shell
scene appearance commands. Docs-only — no code changed.

---

## Verdict: SCENE_SETTINGS_PROTOCOL_SAFE_LOCAL ✅

**Option B**: Define `OP_SCENE_SETTINGS_CMD = 0xFB` as a local constant in
`silk-shell/src/main.rs`. Keep it local until a real Settings app PD exists.
Promote to `crates/sex-pdx/src/lib.rs` when the Settings app needs a shared
constant.

### Why Not A (Defer) or C (Public)

| Option | Rejected Because |
|--------|------------------|
| **A: Defer** | Future Settings app developer would need to add protocol from scratch without a design precedent. Having the command table and handler skeleton defined now reduces future risk. |
| **B: Local** ✅ | **Chosen.** No ABI hash bump. No build spec update. Handler skeleton can be implemented in silk-shell immediately. Promotion to sex-pdx is mechanical when Settings app PD ships. |
| **C: Public sex-pdx** | Requires `ABI_VERSION` bump in `sexos_build_spec.toml`. Unnecessary until a second PD (Settings app) needs the constant. Premature ABI change has real cost (all PDs rebuild). |

---

## Opcode

| Property | Value |
|----------|-------|
| **Opcode** | `OP_SCENE_SETTINGS_CMD = 0xFB` |
| **Scope** | Local `const` in `servers/silk-shell/src/main.rs` |
| **Future** | Promote to `crates/sex-pdx/src/lib.rs` as `pub const` when Settings app is built |

### Why 0xFB

- Confirmed free in both `crates/sex-pdx/src/lib.rs` and `servers/silk-shell/src/main.rs`
- Not adjacent to existing window ops (0xE4-0xE8) or silkbar ops (0xF0-0xF4)
- No collision risk with future opcodes (0xF6-0xFA also free as buffer)

---

## Command Table

`pdx_call(SLOT_SHELL, 0xFB, command_id, value0, value1)`

| ID | Name | value0 | value1 | Mutates | Persists |
|----|------|--------|--------|---------|----------|
| 0x01 | `CMD_SET_PRESET` | `preset_idx` (u8) | 0 | `SCENE_APPEARANCE_STATE.preset_idx`; clears `use_custom_colors`; clears `ACTIVE_TINT_IDX` | ✅ Same as F5 |
| 0x02 | `CMD_CYCLE_PRESET` | 0 | 0 | `preset_idx = (idx + 1) % 4`; clears custom/tint | ✅ Same as F5 |
| 0x03 | `CMD_SET_TINT` | `tint_idx` (u8) | 0 | `ACTIVE_TINT_IDX = tint_idx % 8` | ❌ Ephemeral |
| 0x04 | `CMD_CYCLE_TINT` | 0 | 0 | `ACTIVE_TINT_IDX = (idx + 1) % 8` | ❌ Same as F6 |
| 0x05 | `CMD_SET_CHROME_FLAGS` | `flags` (u8) | 0 | `SCENE_APPEARANCE_STATE.chrome_flags = flags` | ✅ Persisted |
| 0x06 | `CMD_TOGGLE_TOP_BAR` | 0 | 0 | Calls `toggle_top_bar_for_active_frame()` | ❌ Frame flag |
| 0x07 | `CMD_SET_ACCESSIBILITY` | `flags` (u8) | 0 | `SCENE_APPEARANCE_STATE.accessibility_flags = flags` | ✅ Persisted |
| 0x08 | `CMD_RESET_DEFAULTS` | 0 | 0 | `SCENE_APPEARANCE_STATE = DEFAULT_SCENE_APPEARANCE`; `ACTIVE_TINT_IDX = 0` | ✅ Persisted |

All commands trigger `resolve_scene_render_tokens()` → `push_token_preset()` →
`pdx_call(SLOT_DISPLAY, 0xFC, ...)` after mutation.

---

## State Mutation Rules

### Preset commands (0x01, 0x02)

```
SCENE_APPEARANCE_STATE.preset_idx = value0 % PRESET_COUNT
SCENE_APPEARANCE_STATE.use_custom_colors = 0  // clear custom override
ACTIVE_TINT_IDX = 0                           // reset tint
→ resolve, push, persist
```

Setting a preset always clears custom color overrides and tint,
matching existing F5 behavior.

### Tint commands (0x03, 0x04)

```
ACTIVE_TINT_IDX = value0 % TINT_COUNT
// NO change to SCENE_APPEARANCE_STATE
→ resolve, push, NO persist
```

Tint is ephemeral (same as F6). It applies on top of whatever
preset or custom colors are active.

### Chrome flags (0x05)

```
SCENE_APPEARANCE_STATE.chrome_flags = value0 & CHROME_FLAG_MASK
→ resolve, push, persist
```

V1 chrome flags: `bit 0 = top_bar_enabled` (reserved; actual per-frame
top bar still controlled via 0xFD).

### Accessibility (0x07)

```
SCENE_APPEARANCE_STATE.accessibility_flags = value0 & ACCESSIBILITY_FLAG_MASK
→ resolve, push, persist
```

V1 accessibility flags: `bit 0 = high_contrast`, `bit 1 = colorblind_safe`.
Bits are reserved — no behavior implemented yet.

### Reset defaults (0x08)

```
SCENE_APPEARANCE_STATE = DEFAULT_SCENE_APPEARANCE
ACTIVE_TINT_IDX = 0
→ resolve, push, persist
```

Restores BottleGlass preset, clears all custom/tint/flags.

---

## Persistence Rules

| Command | Key | Blob | Mechanism |
|---------|-----|------|-----------|
| `CMD_SET_PRESET` (0x01) | 0x01 | `pack_scene_settings_blob(preset_idx, chrome_flags, access_flags)` | `pdx_call(SLOT_SEXSTORE, OP_KV_PUT, 1, blob, 0)` |
| `CMD_CYCLE_PRESET` (0x02) | 0x01 | Same | Same |
| `CMD_SET_CHROME_FLAGS` (0x05) | 0x01 | Same blob with updated chrome_flags | Same |
| `CMD_SET_ACCESSIBILITY` (0x07) | 0x01 | Same blob with updated access_flags | Same |
| `CMD_RESET_DEFAULTS` (0x08) | 0x01 | Blob for preset_idx=0, flags=0 | Same |
| `CMD_SET_TINT` (0x03) | — | No persist | Ephemeral |
| `CMD_CYCLE_TINT` (0x04) | — | No persist | Ephemeral |
| `CMD_TOGGLE_TOP_BAR` (0x06) | — | No persist (frame flag) | Ephemeral |

### Persistence gap (existing)

`custom_colors[8]` (32 bytes) exceeds the 8-byte blob limit in sexstore V1.
Custom colors are in-memory only. A second sexstore key (0x02) or expanded
value size is needed before custom colors can be persisted. See
`SCENE_SETTINGS_APP_PLAN_V1.md §5` for details.

---

## Handler Skeleton (silk-shell main loop)

```rust
OP_SCENE_SETTINGS_CMD => {
    let cmd = msg.arg0;
    let val = msg.arg1;
    let _flags = msg.arg2;
    match cmd {
        0x01 => { /* CMD_SET_PRESET */ }
        0x02 => { /* CMD_CYCLE_PRESET */ }
        0x03 => { /* CMD_SET_TINT */ }
        0x04 => { /* CMD_CYCLE_TINT */ }
        0x05 => { /* CMD_SET_CHROME_FLAGS */ }
        0x06 => { /* CMD_TOGGLE_TOP_BAR */ }
        0x07 => { /* CMD_SET_ACCESSIBILITY */ }
        0x08 => { /* CMD_RESET_DEFAULTS */ }
        _ => { /* unknown cmd — log and ignore */ }
    }
    pdx_reply(0);
}
```

All arms resolve tokens and push to sexdisplay after mutation.
Persisting commands also call the sexstore PUT path.

---

## Caller Policy (V1)

- **Accepted callers**: Any PD that can reach `SLOT_SHELL` (including future
  Settings app). No authentication in V1.
- **Collar policy**: Deferred. Future versions may gate privileged commands
  (e.g., accessibility, reset) behind a Collar capability.
- **Rate limiting**: None in V1. The handler is synchronous and bounded.

---

## Proof Strategy

### Unit (synthetic)

- Add `OP_SCENE_SETTINGS_CMD` with each command ID to the synthetic keyboard
  proof in `servers/sexinput/src/main.rs` (or to a new synthetic settings app
  proof)
- Verify markers match expected state mutations
- Verify persistence fires for persistable commands

### Integration (runtime)

- Build with `SEXOS_KEYBOARD_PROOF=1` + future `SEXOS_SETTINGS_PROOF=1`
- Run headless QEMU, capture serial log
- Verify: `[shell.scene.settings.protocol] cmd=N ok=1`
- Verify: `[shell.appearance.preset] idx=N` after CMD_SET_PRESET
- Verify: `[sexstore.kv.put] key=1 ok=1` after persistable commands
- Zero faults

### Proof gating

```rust
const SETTINGS_PROTOCOL_PROOF_ENABLED: bool = option_env!("SEXOS_SETTINGS_PROOF").is_some();
```

Follows the same `option_env!` pattern as `KEYBOARD_PROOF_ENABLED` and
`KEYBOARD_CURSOR_ENABLED`.

---

## Files for Implementation

### Modified

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Add `const OP_SCENE_SETTINGS_CMD = 0xFB`; add `CMD_*` constants; add handler arm in main loop; add marker budget; add `[shell.scene.settings.protocol] cmd=N ok=N` |

### NOT modified

- `crates/sex-pdx/src/lib.rs` — Opcode kept local (Option B). Promoted later.
- `sexos_build_spec.toml` — No ABI_VERSION bump.
- `servers/sexdisplay/` — No renderer changes.
- `kernel/` — FORBIDDEN.
- `servers/sexinput/` — Proof in future phase.
- `servers/sexstore/` — Persistence path unchanged.
- `servers/silkbar/` — Unrelated.

---

## STOP Conditions

| Condition | Verdict |
|-----------|---------|
| Requires sex-pdx ABI_VERSION bump | STOP. Local constant sufficient until Settings app exists. |
| Requires kernel changes | STOP. No kernel edits. |
| Requires sexdisplay changes | STOP. protocol is shell-only. |
| Requires heap allocation | Safe. Fixed-size state, no heap. |
| Requires new PD spawn | Safe. Handler runs in existing silk-shell loop. |
| CMD_SET_PRESET with out-of-range idx | Clamp: `val % PRESET_COUNT`. No crash. |
| CMD_SET_TINT with out-of-range idx | Clamp: `val % 8`. No crash. |
| Unknown cmd id | Log `[shell.scene.settings.protocol] cmd=0x?? unknown` + reply 0. No crash. |
| Marker budget exhausted | Stop printing, continue execution. |

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_SETTINGS_APP_PLAN_V1.md` | Phase 0 IPC design (Option B) this protocol plan refines |
| `docs/handoff/SCENE_SETTINGS_PANEL_STATIC_V1.md` | Panel toggle (F7) — already merged |
| `docs/handoff/SCENE_SETTINGS_PANEL_KEYS_V1.md` | Panel keys (1/2/3/Esc) — already merged |
| `docs/handoff/SCENE_SETTINGS_PERSIST_V1.md` | Persistence path reused by protocol |
| `docs/handoff/SCENE_SETTINGS_INMEM_V1.md` | SceneAppearanceState mutated by protocol |
| `servers/silk-shell/src/main.rs` | Target for handler implementation |
| `crates/sex-pdx/src/lib.rs` | Future home for promoted opcode |

## Next Recommended Phase

**SCENE_SETTINGS_PROTOCOL_V1** — Implement `OP_SCENE_SETTINGS_CMD = 0xFB`
handler in silk-shell's main loop. All 8 command IDs. Reuses existing
helpers. Markers. Build + proof. See "Files for Implementation" above
for exact changes.

Or, if protocol implementation is deferred:

**SCENE_SETTINGS_PANEL_SYNTH_PROOF_V1** — Add synthetic F7 + panel key proof
in sexinput. See `SCENE_SETTINGS_APP_PLAN_V1.md §7` for full phase guidance.
