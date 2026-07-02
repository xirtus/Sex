# SCENE_SETTINGS_APP_PLAN_V1

## Status

Design (2026-05-04). Scene Settings application/control-surface architecture for
appearance configuration (colours, opacity, blur, glow, top bar, tab/lights modes,
accessibility, preset selection, per-scope overrides). Docs-only — no code changed.

---

## Verdict: SCENE_SETTINGS_APP_SAFE_TO_DESIGN_WITH_IPC_OPTION_B ✅

All requirements feasible without kernel/ABI changes.

---

## 1. UX Model — Two-Tier

| Tier | Name | Scope | Controls | Labels | Target |
|------|------|-------|----------|--------|--------|
| V1 | SilkBar Quick Settings panel | OS surface 0x96 | Preset grid, color swatches, toggles as colored rects | No text — shape/color affordance | Immediate |
| V2 | Full Scene Settings app | Standalone PDX app surface | Color pickers, sliders, preset browser, a11y section | Labels, descriptions, readouts | Deferred (font pipeline) |

### V1 — Quick Settings panel

Extends the panel toggle pattern (launcher/status/clock/bell) with a dedicated surface:

- **Preset cards**: 3 x colored rect regions — click to select preset
- **Color swatch rows**: Per-row (rim, bar, tab) 8-clickable palette chips
- **Mode toggles**: Circle rects for top bar on/off, tab strip mode, lights mode
- **Accessibility toggles**: Square rects for high contrast, colorblind safe, focus ring, larger targets
- **Preset indicator**: Colored bar + arrow regions — click arrows to cycle preset (F5-equivalent)

All controls use shape/color-only affordances. No text labels in V1.

### V2 — Full Settings app (future)

Standalone PDX server with app surface (id >= 100):
- Tabbed layout: Presets / Colors / Glass / Accessibility
- Preset grid with named cards (font pipeline)
- Color pickers with palette (widget library)
- Sliders for glow, opacity, blur (slider widget)
- Toggle switches for chrome modes and a11y flags
- Scope selector (per-scene, per-monitor, global)

V2 blocked on font/text pipeline and widget library.

---

## 2. Surface ID Allocation

| ID | Surface | Owner | Status |
|----|---------|-------|--------|
| 0x90 | Cursor | OS (shell) | Existing |
| 0x92 | Launcher panel | OS (shell) | Existing |
| 0x93 | Status panel | OS (shell) | Existing |
| 0x94 | Clock panel | OS (shell) | Existing |
| 0x95 | Bell panel | OS (shell) | Existing |
| **0x96** | **Settings panel** | **OS (shell)** | **V1 target — reserved** |
| 0x97 | Standalone Settings app | OS (shell) | V2 target — reserved |
| 100+ | App surfaces | Apps | Existing |

Decision: Use SURFACE_ID_SETTINGS = 0x96 for V1 Quick Settings panel.
Status panel (0x93) remains separate — a gear chip in status can open settings.

---

## 3. Ownership Split

### Invariant

Settings app -> silk-shell -> sexdisplay. Settings app NEVER sends
OP_APPEARANCE_TOKENS. Sexdisplay NEVER receives settings IPC directly.

### Responsibility matrix

| Capability | Settings App | silk-shell | sexdisplay |
|-----------|-------------|------------|------------|
| Render UI surface | (via shell 0xEC) | | composite_pixel |
| Dispatch input | | hit-test | |
| Validate settings | | clamp, range, policy | |
| Apply SceneAppearanceState | | write to static | |
| Resolve + push tokens | | resolve + 0xFC push | |
| Clamp + render tokens | | | clamp, composite_pixel |
| Persist via sexstore | | fire KV PUT | |
| Read default/fallback | | DEFAULT_SCENE_APPEARANCE | DEFAULT_RENDER_TOKENS |
| Collar policy (future) | | gate privileged writes | |

---

## 4. IPC Design

### Options evaluated

| Option | Description | Verdict |
|--------|-------------|---------|
| A | Settings app writes sexstore KV, shell polls | Rejected — polling fragile, race conditions |
| B | New opcode OP_SCENE_SETTINGS_CMD = 0xFB | Recommended — clean, direct, no ABI impact |
| C | Reuse OP_HID_EVENT with custom scancodes | Rejected — misuse of input pipeline |

### Recommended: Option B with opcode 0xFB

Place in crates/sex-pdx/src/lib.rs (shared constant, not ABI — no build spec update).
Alternative: keep local in silk-shell (no sex-pdx change at all).

Command format (single pdx_call, 3 args x u64):

| Cmd ID | Name | value0 | value1 |
|--------|------|--------|--------|
| 0x01 | CMD_SET_PRESET | preset_idx (u8) | 0 |
| 0x02 | CMD_SET_CUSTOM_COLOR | slot_idx (u8) | ARGB (u32) |
| 0x03 | CMD_SET_USE_CUSTOM | 0 or 1 | 0 |
| 0x04 | CMD_SET_ACCESSIBILITY | flags (u8) | 0 |
| 0x05 | CMD_SET_CHROME_FLAGS | flags (u8) | 0 |
| 0x06 | CMD_RESET_TO_DEFAULTS | 0 | 0 |

### STOP conditions

- Requires ABI_VERSION bump: NO — opcodes are protocol constants, not ABI.
- Requires kernel slot/PD change: STOP. Settings app is regular PDX client.
- Conflicts with existing opcode: 0xFB is free (0xF5-0xFB confirmed open).
- Requires sexdisplay changes: STOP. Settings never touches sexdisplay.

### Handler pseudocode (silk-shell main loop)

Match on 0xFB:
- CMD_SET_PRESET: validate idx < PRESET_COUNT, update SceneAppearanceState.preset_idx,
  set use_custom_colors=0, ACTIVE_TINT_IDX=0, resolve+push tokens, persist via sexstore
- CMD_SET_CUSTOM_COLOR: validate slot < 8, set custom_colors[slot] = argb,
  set use_custom_colors=1, resolve+push tokens
- CMD_SET_ACCESSIBILITY: set accessibility_flags = flags, resolve+push tokens
- CMD_RESET_TO_DEFAULTS: SCENE_APPEARANCE_STATE = DEFAULT_SCENE_APPEARANCE,
  ACTIVE_TINT_IDX = 0, resolve+push tokens, persist via sexstore
- Unknown cmd id: log marker [shell.scene.settings.cmd] cmd=unknown

All commands trigger the same resolve+push+persist path that F5 already uses.

---

## 5. Storage Interaction

### Current persist path (SCENE_SETTINGS_PERSIST_V1)

SceneAppearanceState (silk-shell static) -> resolve_scene_render_tokens()
-> push_token_preset() -> 0xFC -> sexdisplay
On F5 change: also sexstore K/V PUT (key 0x01, 8-byte blob)

### With settings app

Settings app -> OP_SCENE_SETTINGS_CMD -> shell handler
-> updates SceneAppearanceState
-> resolve -> push -> 0xFC -> sexdisplay
-> sexstore K/V PUT (same path as F5)

### Persistence gap

custom_colors[8] = 32 bytes exceeds current 8-byte blob. Gap must be resolved
before custom colors can be persisted:
- Short-term: second sexstore key 0x02 for custom colors
- Long-term: expand sexstore value size > 8 bytes

Not a V1 blocker (in-memory custom colors work without persistence).

---

## 6. Blockers

| Blocker | Severity | Impact | V1 Workaround |
|---------|----------|--------|---------------|
| No font/text pipeline | HIGH | Cannot label controls | Shape/color affordances only |
| No control widget library | MEDIUM | No sliders/buttons | SilkBar-style rect hit-testing |
| No OS panel keyboard focus model | MEDIUM | Tab/arrow nav impossible | V1 pointer-only |
| No per-scene/per-monitor scope model | LOW | Global-only settings | Sufficient for V1 |
| Custom colors not persisted | LOW | Lost on reboot | In-memory works for V1 |

### Font pipeline (HIGH)

No glyph atlas, font loader, text compositing exists anywhere in the stack.
SilkBar clock uses hardcoded 7-segment pixel patterns, not real text.
Blocks: labeled preset cards, value readouts, tab bar labels, descriptions.

### Control library (MEDIUM)

No button, slider, switch, or color-picker primitives. Only interactive
surfaces are SilkBar chips (rect hit-tests) and OS panels (solid rects).
Workaround: V1 maps fixed-position rects -> commands via geometry constants
+ hit-test in silk-shell (same pattern as SilkBar hit_test_action()).

---

## 7. Phase Plan

### Phase 0 — Static settings surface + IPC (next)

Goal: colored rect settings panel with clickable regions. No labels.

Changes (silk-shell only):
1. Add SURFACE_ID_SETTINGS = 0x96 and SETTINGS_ACTIVE bool
2. Add local OP_SCENE_SETTINGS_CMD handler (or sex-pdx const 0xFB)
3. Add settings panel toggle (F7 key or SilkBar gear chip)
4. Define control region geometry (3-4 rects: preset up, preset down, reset)
5. Add handle_settings_click(px, py) -> rect hit-test -> cmd dispatch
6. Wire into shell click dispatch (before app surface dispatch)
7. Add synthetic click proof in sexinput

Forbidden: sexdisplay changes, text/labels, keyboard navigation, persistence changes.
Dependencies: None. All capabilities exist.

### Phase 1 — Keyboard controls

Add SETTINGS_FOCUSED_CONTROL static, Tab cycle through regions,
Enter/Space activation, arrow value changes. Focus ring via brighter border.
Depends on Phase 0.

### Phase 2 — Pointer controls (swatches, sliders)

Expand to 12+ control regions. Swatch click -> CMD_SET_CUSTOM_COLOR.
Slider rects with horizontal position tracking for continuous values.
Depends on Phase 0 (IPC handler exists).

### Phase 3 — Persistence upgrade

Second sexstore key 0x02 for custom_colors[8] blob.
Write on CMD_SET_CUSTOM_COLOR. Read + restore on boot.
Depends on Phase 2 + sexstore value size support.

### Phase 4 — Full standalone settings app

New PDX server (servers/sexsettings/) with labeled controls.
Font pipeline required. SLOT_SHELL capability at spawn.
Depends on font pipeline, widget library, app spawn infrastructure.


---

## 8. Phase Guidance

| Priority | Phase | Effort | Value | Blocked by |
|----------|-------|--------|-------|------------|
| 1 | Phase 0 (static surface + IPC) | Low | High | Nothing |
| 2 | Phase 1 (keyboard controls) | Medium | Medium | Panel focus model |
| 3 | Phase 2 (pointer controls) | Medium | High | Phase 0 |
| 4 | Phase 3 (persistence upgrade) | Low | Medium | sexstore value size |
| -- | Phase 4 (full app) | High | High | Font pipeline |

### Recommended next: Phase 0

Build the minimal settings control surface:
1. SURFACE_ID_SETTINGS = 0x96, SETTINGS_ACTIVE toggle
2. OP_SCENE_SETTINGS_CMD handler (CMD_SET_PRESET, CMD_RESET_TO_DEFAULTS)
3. Toggle via F7 keyboard shortcut
4. 3-4 clickable control regions on settings surface
5. Wire clicks to commands: region click -> CMD_SET_PRESET / CMD_RESET
6. Synthetic click proof in sexinput
7. Build + verify markers

Only silk-shell changes. No sexdisplay, sex-pdx ABI, kernel, or sexstore changes.

---

## 9. SilkBar Alternatives

### Gear chip in SilkBar

Add settings chip between Bell and Clock (same pattern as existing chips).
Click opens settings panel 0x96. No architecture changes. Deferrable.

### Status panel section

Add settings controls at bottom of status panel (0x93). Reuses existing
toggle pattern but mixes system status with settings. Verdict: Defer.
V1 gets dedicated surface for clean separation.

---

## 10. STOP Conditions

| Condition | Verdict |
|-----------|---------|
| Settings app writes framebuffer directly | STOP. sexdisplay sole FB writer. |
| Settings app sends OP_APPEARANCE_TOKENS | STOP. Shell owns policy. |
| New opcode requires ABI_VERSION bump | Safe. Opcodes are protocol constants. |
| Requires sexdisplay changes | STOP. Settings never touches sexdisplay. |
| Requires kernel changes | STOP. No kernel edits. |
| Requires heap for control state | Safe. Fixed-size statics only. |
| Font pipeline needed for V1 | Conditional. V1 uses shape/color only. |
| Changes silkbar model or ABI | STOP. Settings is shell-owned surface. |

---

## 11. Deferred Proof Items

| Item | Blocked by | Target |
|------|-----------|--------|
| Settings panel toggle proof (0x96) | Nothing | Phase 0 |
| OP_SCENE_SETTINGS_CMD end-to-end | Nothing | Phase 0 |
| Click-to-preset proof | Nothing | Phase 0 |
| Keyboard nav of OS panels | Panel focus model | Phase 1 |
| Color swatch -> custom color proof | Phase 0 IPC handler | Phase 2 |
| Slider drag -> glow intensity proof | Slider rendering | Phase 2 |
| Full a11y flags editing proof | Phase 2 handlers | Phase 2 |
| Custom color persistence | sexstore value > 8 bytes | Phase 3 |
| Labeled controls proof | Font pipeline | Phase 4 |
| Per-scene settings model | Scene struct design | V2 |
| Per-monitor settings model | Monitor config design | V2 |
| Standalone settings PD spawn | Font pipeline + app binary | Phase 4 |

---

## 12. File Impact (Phase 0)

### Modified

| File | Changes |
|------|---------|
| servers/silk-shell/src/main.rs | SURFACE_ID_SETTINGS, SETTINGS_ACTIVE, OP_SCENE_SETTINGS_CMD handler, control region geometry, handle_settings_click(), F7 binding |
| servers/sexinput/src/main.rs | Optional: synthetic click proof for settings panel + preset selection |

### Created

| File | Role |
|------|------|
| docs/handoff/SCENE_SETTINGS_APP_PLAN_V1.md | This document |
| docs/handoff/SCENE_SETTINGS_APP_V1.md | Future Phase 0 implementation handoff |

### NOT modified (all phases)

| File | Reason |
|------|--------|
| servers/sexdisplay/src/main.rs | No renderer changes |
| servers/silkbar/ | Settings is shell-owned, not SilkBar module |
| crates/silkbar-model/ | No ABI/theme changes |
| crates/sex-pdx/src/lib.rs | Optional -- can keep opcode local |
| servers/sexusb/ | Unrelated |
| servers/sexstore/ | Persistence path unchanged |
| kernel/ | FORBIDDEN |
| sexos_build_spec.toml | No ABI hash change |

---

## 13. References

| Doc | Relevance |
|-----|-----------|
| SCENE_SETTINGS_STORAGE_PLAN_V1.md | Model split: Intent -> State -> Tokens -> Blob -> App |
| SCENE_SETTINGS_INMEM_V1.md | SceneAppearanceState, resolve_scene_render_tokens |
| SCENE_SETTINGS_PERSIST_V1.md | Current persistence (8-byte blob, key 0x01) |
| SCENE_SETTINGS_INPUT_PROOF_V1.md | F5/F6 keyboard proof precedent |
| SCENE_RENDER_TOKENS_V1.md | OP_APPEARANCE_TOKENS = 0xFC, two-call IPC |
| SCENE_RENDER_TOKEN_PRESETS_V1.md | TOKEN_PRESETS, F5 cycling |
| SCENE_CUSTOM_COLOR_KEYS_V1.md | F6 tint cycling, custom_colors exercise |
| SILKBAR_CLICKABLE_CONTROLS_V1.md | SilkBar hit-test pattern (rect-based controls) |
| SILKBAR_CLOCK_PANEL_V1.md | Panel toggle pattern (0xEC/0xEE) |
| PANEL_TOGGLE_CONSOLIDATION_V1.md | toggle_os_panel() helper pattern |
| SILK_CHROME_SETTINGS_PLAN_V1.md | Chrome settings roadmap |
| SCENE_APPEARANCE_CONTROLS_PLAN_V1.md | Control taxonomy, effect blocker rules |
| SHELL_GLOBAL_INTERACTION_CONTRACT_V1.md | Subcontract A-G |
| STABLE_BASELINE_20260503.md | Locked invariants, OS Surface ID registry |

---

## 14. Pass Criteria

- [x] Verdict: SCENE_SETTINGS_APP_SAFE_TO_DESIGN_WITH_IPC_OPTION_B
- [x] UX model: SilkBar Quick Settings (V1) + Full App (V2)
- [x] UI layout described for both V1 and V2
- [x] Surface ID allocated (0x96 = settings panel)
- [x] Ownership split documented (settings -> shell -> display)
- [x] Responsibility matrix complete
- [x] IPC options evaluated (A: sexstore, B: new opcode, C: HID injection)
- [x] Recommended IPC: OP_SCENE_SETTINGS_CMD with command payload format
- [x] STOP conditions for IPC documented
- [x] Storage interaction defined (reuses existing sexstore path)
- [x] Blockers identified: font pipeline (HIGH), control library (MEDIUM), focus model (MEDIUM), scope model (LOW)
- [x] 5 implementation phases with dependencies and blocker assessments
- [x] Phase selection guidance with priority ordering
- [x] SilkBar quick control alternatives evaluated
- [x] Deferred proof sprint items documented
- [x] File impact summary for Phase 0
- [x] References to existing handoffs complete
- [x] Next recommended phase: Phase 0 (static settings surface + IPC opcode)

---

## Next Recommended Phase

Phase 0 -- Static settings surface + IPC opcode. Build the minimal settings
panel with clickable colored rect regions and OP_SCENE_SETTINGS_CMD handler.
silk-shell only. See file impact table for exact changes.
