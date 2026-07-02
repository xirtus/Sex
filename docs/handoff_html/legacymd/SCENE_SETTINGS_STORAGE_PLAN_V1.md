# SCENE_SETTINGS_STORAGE_PLAN_V1

## Status

Design (2026-05-04). Scene Appearance settings model: V1 in-memory static state, future persistent storage path. Docs-only — no code changed.

---

## Verdict: SCENE_SETTINGS_STORAGE_SAFE_TO_DESIGN ✅

| Requirement | Feasible? | Notes |
|-------------|-----------|-------|
| V1 static model (no storage dep) | ✅ | Extend current `ACTIVE_PRESET_IDX` pattern |
| No heap | ✅ | Fixed-size structs only |
| No new opcode | ✅ | Reuse OP_APPEARANCE_TOKENS = 0xFC |
| No sexfiles/sexstore dependency in V1 | ✅ | Both are stubs or complex; defer |
| Future persistent path feasible | ✅ Conditional | Requires sexstore K/V or sexfiles blob write |
| Corruption/absence fallback | ✅ | Built-in defaults always valid |
| Forward-compatible schema | ✅ | Version byte + reserved fields |

---

## Storage Infrastructure Audit

| Service | State | Usable for settings now? |
|---------|-------|--------------------------|
| `servers/sexstore` | **Stub** — 24-line infinite loop, no PDX, no API | ❌ No |
| `servers/sexfiles` | Phase 19 trampoline VFS — complex, no simple KV | ❌ Not for simple blob in V1 |
| silk-shell statics | `static mut` pattern used everywhere (FRAMES, WINDOWS, etc.) | ✅ V1 |

**Conclusion:** V1 storage is in-memory silk-shell statics only. Persistent storage deferred until sexstore gains a K/V read/write API.

---

## V1 Static State Model

### Current state (implemented, silk-shell)

```rust
// Raw, minimal — just an index
static TOKEN_PRESETS: [[u32; 8]; 4] = [ ... ];
static mut ACTIVE_PRESET_IDX: u8 = 0;
```

Gaps: no chrome flags, no custom color override, no per-scene state, no version tracking.

### V1 target: `SceneAppearanceState`

Replace the raw index + preset table with a named struct in silk-shell. All fields `static mut`. No heap.

```rust
/// Runtime appearance state. No persistence in V1.
/// Initialized to BottleGlass defaults at compile time.
struct SceneAppearanceState {
    /// Active built-in preset index (0..PRESET_COUNT-1).
    /// Drives TOKEN_PRESETS lookup on preset cycle.
    preset_idx: u8,

    /// If true, custom_colors overrides the active preset's colors.
    /// F5 cycling clears this flag (reverts to preset).
    use_custom_colors: bool,

    /// Custom color override (8 × u32, same layout as TokenPreset).
    /// Only meaningful when use_custom_colors = true.
    custom_colors: [u32; 8],

    /// Chrome flags (bit 0 = reserved; top bar is per-frame via 0xFD, not here).
    /// Reserved for future chrome density, rim thickness, tab strip mode, etc.
    chrome_flags: u8,

    /// Accessibility flags (bit 0 = high_contrast, bit 1 = colorblind_safe,
    /// bit 2 = stronger_focus_ring, bit 3 = larger_targets).
    /// V1: passed as-is in appearance_flags field of OP_APPEARANCE_TOKENS.
    accessibility_flags: u8,

    /// Schema version for future migration.
    /// V1 = 1. Increment when fields are added/removed.
    _version: u8,

    /// Reserved for future fields (rim thickness, tab strip mode, etc.).
    _reserved: [u8; 5],
}

static mut SCENE_APPEARANCE: SceneAppearanceState = SceneAppearanceState {
    preset_idx: 0,
    use_custom_colors: false,
    custom_colors: [0u32; 8],
    chrome_flags: 0,
    accessibility_flags: 0,
    _version: 1,
    _reserved: [0u8; 5],
};
```

**Size:** 1 + 1 + 32 + 1 + 1 + 1 + 5 = **42 bytes**. No padding issues (u32 array starts at offset 2 → needs alignment; use `#[repr(C)]` and reorder or accept 2-byte gap). Practical: all fields are u8 except `custom_colors` — put the array at offset 0 or reorder to avoid gaps. Final exact layout: compiler handles it; size doesn't matter since it's a static not sent over wire.

### Token resolution (at send time)

```rust
unsafe fn resolve_tokens() -> [u32; 8] {
    if SCENE_APPEARANCE.use_custom_colors {
        SCENE_APPEARANCE.custom_colors
    } else {
        TOKEN_PRESETS[SCENE_APPEARANCE.preset_idx as usize]
    }
}
```

`push_token_preset` stays as the IPC primitive. Boot and cycle calls go through `resolve_tokens()`.

### Updated cycle behavior

```rust
unsafe fn cycle_scene_render_token_preset() {
    SCENE_APPEARANCE.preset_idx =
        (SCENE_APPEARANCE.preset_idx + 1) % PRESET_COUNT as u8;
    SCENE_APPEARANCE.use_custom_colors = false; // preset cycle clears custom
    let tokens = resolve_tokens();
    push_token_preset(&tokens);
    // budgeted marker...
}
```

---

## Future Persistent Model

### Serializable settings blob

When sexstore gains a K/V API, silk-shell writes a fixed-size blob on any settings change and reads it at boot.

```rust
/// Serializable settings blob for persistent storage.
/// Written to sexstore key "scene.appearance.v1" (or similar).
/// Must fit in a single small fixed-size buffer (≤ 64 bytes total).
#[repr(C)]
struct SceneSettingsBlob {
    magic: [u8; 4],        // b"SCAP" — detects wrong key / corrupt read
    version: u8,           // 1 = V1 layout
    preset_idx: u8,        // 0..PRESET_COUNT-1
    use_custom: bool,      // use custom_colors if true
    chrome_flags: u8,      // reserved in V1
    accessibility_flags: u8,
    _pad: [u8; 3],         // align custom_colors to 4
    custom_colors: [u32; 8], // 32 bytes; only valid if use_custom=true
    checksum: u32,         // CRC32 or simple XOR over bytes[0..56]
}
// Total: 4 + 1 + 1 + 1 + 1 + 1 + 3 + 32 + 4 = 48 bytes.
```

### Corruption and absence handling

| Condition | Action |
|-----------|--------|
| Key not found in sexstore | Use `SCENE_APPEARANCE` defaults (BottleGlass) |
| `magic` mismatch | Discard, use defaults, log warning |
| `version` unknown | Discard, use defaults, log warning |
| `checksum` mismatch | Discard, use defaults, log warning |
| `preset_idx` out of range | Clamp to 0, log warning |
| `custom_colors` contains zero (transparent black) | `clamp_color_token()` in sexdisplay already handles this on receive |

**Invariant:** Boot MUST succeed with defaults if storage is absent or corrupt. Settings failure is never fatal.

### Persistent write trigger

Settings are written to sexstore after any user-driven change (F5 preset cycle, future settings app modification). Write is fire-and-forget — no blocking wait. If sexstore is unavailable (not yet started), the write is dropped silently; defaults apply on next boot.

---

## Model Split

| Type | Owner | Purpose | Location |
|------|-------|---------|----------|
| `AppearanceIntent` | silk-shell (future) | User-facing: preset name, color picker values, accessibility toggles | silk-shell runtime state |
| `SceneAppearanceState` | silk-shell | Runtime state: active preset index, custom override flag, color array, flags | `static mut SCENE_APPEARANCE` in silk-shell |
| `RenderTokensV1` | sexdisplay | Renderer-safe clamped output; populated from IPC | `static mut DISPLAY_TOKENS` in sexdisplay |
| `SceneSettingsBlob` | silk-shell ↔ sexstore | Persistent serialized snapshot; written on change, read at boot | sexstore K/V blob |
| Settings app (future) | new PDX server | UI for AppearanceIntent editing | future `servers/sexsettings` or shell extension |

**Pipeline:** User input → `AppearanceIntent` edit → `SceneAppearanceState` update → `resolve_tokens()` → `push_token_preset()` → `OP_APPEARANCE_TOKENS` IPC → sexdisplay clamp → `DISPLAY_TOKENS` → `composite_pixel()` → framebuffer.

---

## Precedence Rules

Listed low-to-high priority (higher overrides lower):

| Level | Scope | Override |
|-------|-------|----------|
| 1. Global default | Built-in `TOKEN_PRESETS[0]` (BottleGlass) | Applied if no settings exist |
| 2. Preset selection | `TOKEN_PRESETS[preset_idx]` | Replaces global default |
| 3. Custom color override | `SCENE_APPEARANCE.custom_colors` | Replaces preset when `use_custom_colors = true` |
| 4. Per-Scene override (future) | Scene struct `.appearance` | Replaces global when switching scenes |
| 5. Per-monitor override (future) | Monitor config `.appearance_override` | For display-specific values only (e.g. brightness-adjusted colors) |
| 6. Frame-level override (deferred) | `ShellFrame.chrome_flags` | Top bar per-frame (via 0xFD, already implemented); other chrome deferred |

**Rules:**
- Global default applies when no preset set and no custom colors.
- Preset cycle clears custom override (reverts to preset).
- Future: switching scenes pushes that scene's appearance state if it has one; otherwise global applies.
- Per-monitor override only affects display-specific values — not semantic light colors.
- sexdisplay always clamps on receive regardless of source — defense in depth.

---

## Ownership Boundaries

| What | Who owns it | Who may change it |
|------|-------------|-------------------|
| `DISPLAY_TOKENS` in sexdisplay | sexdisplay (sole FB writer) | Written via OP_APPEARANCE_TOKENS from silk-shell only |
| `SCENE_APPEARANCE` in silk-shell | silk-shell | F5 cycle, future settings app via IPC to shell |
| `TOKEN_PRESETS` table | silk-shell (read-only) | Compile-time only |
| Persistent blob in sexstore | sexstore | Written by silk-shell; read-only for all others |
| Renderer clamp (`clamp_color_token`) | sexdisplay | Never bypassed; applied on every receive |

**Invariants:**
- No app sends OP_APPEARANCE_TOKENS directly — shell owns the IPC.
- No app reads DISPLAY_TOKENS — sexdisplay owns the FB.
- sexdisplay clamps regardless of sender trust level.
- Custom colors from a future settings app go through shell, which validates range before sending.

---

## Implementation Phases

### Phase 1 (next): SCENE_SETTINGS_INMEM_V1

Goal: Upgrade from raw `ACTIVE_PRESET_IDX` to `SceneAppearanceState`. No storage dependency.

Changes to `servers/silk-shell/src/main.rs`:
- Replace `ACTIVE_PRESET_IDX: u8` with `SCENE_APPEARANCE: SceneAppearanceState`
- Add `resolve_tokens() -> [u32; 8]`
- Update `cycle_scene_render_token_preset()` to go through `resolve_tokens()`
- Update boot call to use `resolve_tokens()`
- No sexdisplay/sex-pdx/kernel changes

Build impact: silk-shell only. No ABI hash update.

### Phase 2 (design): SEXSTORE_KV_API_PLAN_V1

Goal: Design a minimal K/V read/write API for sexstore. Required before any persistent settings.

Questions to answer:
- What opcode space does sexstore use?
- What is the max value size?
- Is write synchronous or fire-and-forget?
- What happens if sexstore not yet running at write time?

Not design here — this is a separate sexstore phase.

### Phase 3 (implement): SCENE_SETTINGS_PERSIST_V1

Goal: Write `SceneSettingsBlob` to sexstore on change; read and restore at boot.

Prerequisites: Phase 1 + Phase 2 complete. Requires sexstore K/V write/read PDX API.

Changes: silk-shell gains `save_scene_settings()` and `load_scene_settings()` helpers.

### Phase 4 (future): SCENE_SETTINGS_APP_PLAN_V1

Goal: Design settings app UI and IPC. Requires color picker model, settings panel surface, font/text pipeline for labels.

Not blocked by Phase 1 or 2 — can be designed in parallel.

---

## Forbidden Files (for all phases)

- `kernel/` — no kernel changes at any phase
- `servers/sexdisplay/src/main.rs` — no changes needed (0xFC handler is general)
- `crates/sex-pdx/src/lib.rs` — no new opcode needed
- `servers/silkbar/` — independent theme system
- `crates/silkbar-model/` — independent
- `servers/sexusb/` — untouched
- `servers/sexinput/` — untouched

---

## STOP Conditions

| Condition | Action |
|-----------|--------|
| sexstore PDX API does not exist | STOP Phase 3 — design sexstore first (Phase 2) |
| sexfiles VFS path needed for blob | STOP — sexfiles is complex; prefer sexstore simple K/V |
| `SceneAppearanceState` struct requires heap | STOP — must be fixed-size, no Vec |
| Custom color validation needs sexdisplay feedback | OK — sexdisplay clamps on receive; shell clamps before sending too |
| Per-scene appearance needs Scene struct | Design Scene struct first before per-scene overrides |
| Settings app needs text/font pipeline | STOP — defer Settings app until font pipeline exists |

---

## Pass Criteria

- [x] Verdict: SCENE_SETTINGS_STORAGE_SAFE_TO_DESIGN
- [x] V1 static model designed (`SceneAppearanceState`)
- [x] Future persistent model designed (`SceneSettingsBlob`)
- [x] Storage infrastructure audited (sexstore=stub, sexfiles=complex, deferred)
- [x] Model split documented (Intent / State / Tokens / Blob / App)
- [x] Precedence rules specified (6 levels, global→monitor)
- [x] Corruption/absence fallback: always use built-in defaults
- [x] Ownership boundaries defined (shell owns send, sexdisplay owns FB/clamp)
- [x] 4 implementation phases named with dependencies
- [x] Forbidden files identified
- [x] STOP conditions documented
- [x] Next phase: SCENE_SETTINGS_INMEM_V1

---

## Next Phase: SCENE_SETTINGS_INMEM_V1

Implement `SceneAppearanceState` in silk-shell:

1. Define `SceneAppearanceState` struct (42 bytes, no heap)
2. Replace `static mut ACTIVE_PRESET_IDX: u8` with `static mut SCENE_APPEARANCE: SceneAppearanceState`
3. Add `resolve_tokens() -> [u32; 8]` helper
4. Update `cycle_scene_render_token_preset()` to use `SCENE_APPEARANCE.preset_idx` and `resolve_tokens()`
5. Update boot `send_scene_render_tokens()` to use `resolve_tokens()`
6. Build: `./scripts/entrypoint_build.sh`
7. Verify: boot teal, F5 cycles presets as before, no new behavior regression
8. Create `docs/handoff/SCENE_SETTINGS_INMEM_V1.md`

Only silk-shell changes. No sexdisplay, sex-pdx, or kernel changes.

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_APPEARANCE_CONTROLS_PLAN_V1.md` | Original model candidates (Candidate A/B, SceneAppearance struct) |
| `docs/handoff/SCENE_RENDER_TOKENS_V1.md` | Token IPC and DISPLAY_TOKENS implementation |
| `docs/handoff/SCENE_RENDER_TOKEN_PRESETS_V1.md` | Current preset cycling implementation |
| `docs/handoff/SILK_CHROME_SETTINGS_PLAN_V1.md` | Chrome settings roadmap, scope definitions |
| `servers/silk-shell/src/main.rs` | `TOKEN_PRESETS`, `ACTIVE_PRESET_IDX`, `push_token_preset` |
| `servers/sexdisplay/src/main.rs` | `DISPLAY_TOKENS`, `clamp_color_token`, 0xFC handler |
| `servers/sexstore/src/main.rs` | Stub (24 lines, infinite loop) — not usable yet |
| `servers/sexfiles/src/main.rs` | Phase 19 VFS — complex, not suited for simple KV in V1 |
