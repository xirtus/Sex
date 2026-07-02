# SCENE_SETTINGS_PROTOCOL_SYNTH_PROOF_V1

## Status

Complete (2026-05-04). Local `OP_SCENE_SETTINGS_CMD = 0xFB` command handler
proven via gated synthetic proof in silk-shell. All 5 command stages verified.
Zero faults.

---

## Changed Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Added `SCENE_SETTINGS_PROTOCOL_PROOF_ENABLED` const (env-var gated), `SCENE_SETTINGS_PROTOCOL_PROOF_STAGE` static, synthetic proof block with 5 stages before main loop dispatch. ~50 lines. |
| `docs/handoff/SCENE_SETTINGS_PROTOCOL_SYNTH_PROOF_V1.md` | New — this document |

### NOT modified

- `crates/sex-pdx/` — no ABI change
- `kernel/` — no kernel changes
- `servers/sexdisplay/` — no renderer changes
- `servers/sexinput/` — no input changes
- `servers/sexstore/` — no persistence changes

---

## Proof Gate

```rust
const SCENE_SETTINGS_PROTOCOL_PROOF_ENABLED: bool =
    option_env!("SEXOS_SCENE_SETTINGS_PROTOCOL_PROOF").is_some();
```

Default build (no env var): zero behavior change. Proof build:
`SEXOS_SCENE_SETTINGS_PROTOCOL_PROOF=1 ./scripts/entrypoint_build.sh`

---

## Command Sequence

| Stage | Cmd | Action | Marker | Persists? |
|-------|-----|--------|--------|-----------|
| 0 | `CMD_SET_PRESET` (1) | Set preset_idx=1, clear custom/tint, push tokens | `cmd=1 preset=1 ok=1` | ✅ PUT |
| 1 | `CMD_CYCLE_TINT` (4) | Cycle tint index, push tokens | `cmd=4 ok=1` | ❌ |
| 2 | `CMD_TOGGLE_TOP_BAR` (6) | Toggle top bar on active frame | `cmd=6 ok=1` | ❌ |
| 3 | `CMD_RESET_DEFAULTS` (8) | Reset to BottleGlass, clear custom/tint/flags | `cmd=8 ok=1` | ✅ PUT |
| 4 | invalid (99) | Unknown cmd — ignored safely | `cmd=99 ok=0 unknown` | ❌ |

---

## Proven Markers

| Marker | Count | Meaning |
|--------|-------|---------|
| `[shell.scene.settings.cmd.proof] stage=0..4` | 5 | Each proof stage fired ✅ |
| `[shell.scene.settings.cmd] cmd=1 preset=1 ok=1` | 1 | CMD_SET_PRESET with idx=1 ✅ |
| `[shell.scene.settings.cmd] cmd=4 ok=1` | 1 | CMD_CYCLE_TINT ✅ |
| `[shell.scene.settings.cmd] cmd=6 ok=1` | 1 | CMD_TOGGLE_TOP_BAR ✅ |
| `[shell.scene.settings.cmd] cmd=8 ok=1` | 1 | CMD_RESET_DEFAULTS ✅ |
| `[shell.scene.settings.cmd] cmd=99 ok=0 unknown` | 1 | Invalid cmd rejected safely ✅ |
| `[shell.appearance.custom] mode=tint tint=1` | 1 | Tint applied after CYCLE_TINT ✅ |
| `[shell.frame.topbar.toggle] frame=1 enabled=0` | 1 | Top bar toggled after CMD_TOGGLE_TOP_BAR ✅ |
| `[sexstore.kv.put] key=1 ok=1` | 2 | Two persist calls (SET_PRESET + RESET_DEFAULTS) ✅ |
| panic / #PF / #GP | **0** | ✅ **No faults** |

---

## Build

```
# Default build
./scripts/entrypoint_build.sh
[SEXOS ENTRYPOINT] success

# Proof build
SEXOS_SCENE_SETTINGS_PROTOCOL_PROOF=1 ./scripts/entrypoint_build.sh
[SEXOS ENTRYPOINT] success
```

Both builds pass. No new warning types. No ABI hash update.

---

## Runtime Proof Log (headless QEMU)

```
642  [shell.scene.settings.cmd.proof] stage=0
643  [shell.scene.settings.cmd] cmd=1 preset=1 ok=1
644  [shell.scene.settings.cmd.proof] stage=1
645  [shell.appearance.custom] mode=tint tint=1
646  [shell.scene.settings.cmd] cmd=4 ok=1
647  [shell.scene.settings.cmd.proof] stage=2
648  [shell.frame.tab.info.send] frame=1 surface=100 tabs=2 active=0 chrome=0
649  [shell.frame.topbar.toggle] frame=1 enabled=0
650  [shell.scene.settings.cmd] cmd=6 ok=1
651  [shell.scene.settings.cmd.proof] stage=3
652  [shell.scene.settings.cmd] cmd=8 ok=1
653  [shell.scene.settings.cmd.proof] stage=4
654  [shell.scene.settings.cmd] cmd=99 ok=0 unknown
...
809  [sexstore.kv.put] key=1 ok=1
810  [sexstore.kv.put] key=1 ok=1
```

---

## Limitations

| Limitation | Impact |
|------------|--------|
| **Synthetic proof, not real Settings app** | Proves handler logic but does not exercise PD→PD IPC dispatch |
| **No caller policy** | Collar capability gating deferred |
| **No SET_CHROME_FLAGS or SET_ACCESSIBILITY proven** | Only 4 of 8 commands tested; remaining 4 are structurally identical |
| **Persistence verified via sexstore marker** | PUT is fire-and-forget; no end-to-end ack verified |

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_SETTINGS_PROTOCOL_V1.md` | Handler implementation under test |
| `docs/handoff/SCENE_SETTINGS_PROTOCOL_PLAN_V1.md` | Protocol design this proves |
| `docs/handoff/SCENE_SETTINGS_PERSIST_V1.md` | Persistence path reused |
| `docs/handoff/SCENE_SETTINGS_PANEL_KEYS_V1.md` | Panel keyboard controls (separate path) |

## Next Recommended Phase

**SCENE_SETTINGS_PANEL_CONTROLS_PLAN_V1** — Design clickable controls for the
Scene Settings panel surface (preset grid, tint swatches, top bar toggle).
After that: implementation of rect-based hit-test + dispatch via protocol commands.
