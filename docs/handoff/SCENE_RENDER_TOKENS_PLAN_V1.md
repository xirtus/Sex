# SCENE_RENDER_TOKENS_PLAN_V1

## Status

Design (2026-05-04). Renderer-safe Scene RenderTokens struct and IPC protocol from silk-shell to sexdisplay. Docs-only — no code changed.

---

## Verdict: SCENE_RENDER_TOKENS_SAFE_WITH_NEW_OPCODE ✅

| Requirement | Feasible? | How |
|-------------|-----------|-----|
| Define RenderTokens struct | ✅ | Fixed-size, no_std-safe, 8 × u32 + 2 × u8 |
| Clamping rules | ✅ | Alpha forced to 0xFF, valid range for each token |
| IPC protocol | ✅ | New opcode `OP_APPEARANCE_TOKENS = 0xFC` in sex-pdx |
| No dynamic allocation | ✅ | Static token table in sexdisplay, arg-packed IPC |
| No renderer changes in this phase | ✅ | Docs-only |
| No kernel/ABI changes | ✅ | Userland opcode, no ABI_VERSION bump |
| Next phase named | ✅ | SCENE_RENDER_TOKENS_V1 |

---

## Opcode Space Audit

### Used opcodes (collision check)

| Opcode | Constant | Owner | Slot |
|--------|----------|-------|------|
| `0x14` | `OP_SHELL_BIND_BUFFER` | silk-shell | SLOT_DISPLAY |
| `0x15` | `OP_DISPLAY_SET_SNAPSHOT` | silk-shell | SLOT_DISPLAY |
| `0xE4` | `OP_WINDOW_CREATE` | sex-pdx (shared) | varies |
| `0xE5` | `OP_WINDOW_SUBMIT` | sex-pdx (shared) | varies |
| `0xE6` | `OP_WINDOW_VBLANK` | sex-pdx (shared) | varies |
| `0xE7` | `OP_WINDOW_MAP` | sex-pdx (shared) | varies |
| `0xE8` | `OP_WINDOW_WRITE` | sex-pdx (shared) | varies |
| `0xEB` | `OP_SURFACE_UPDATE` | silk-shell (local) | SLOT_DISPLAY |
| `0xEC` | `OP_SURFACE_UPSERT` | silk-shell (local) | SLOT_DISPLAY |
| `0xED` | `OP_SET_FOCUS` | silk-shell (local) | SLOT_DISPLAY |
| `0xEE` | `OP_SURFACE_DESTROY` | silk-shell (local) | SLOT_DISPLAY |
| `0xEF` | `OP_SURFACE_FILL_RECT` | silk-shell (local) | SLOT_DISPLAY |
| `0xF0` | `OP_SILKBAR_PING` | sex-pdx (shared) | SLOT_SILKBAR |
| `0xF1` | `OP_SILKBAR_GET_ABI` | sex-pdx (shared) | SLOT_SILKBAR |
| `0xF2` | `OP_SILKBAR_UPDATE` | sex-pdx (shared) | SLOT_SILKBAR |
| `0xF3` | `OP_SILKBAR_WORKSPACE_ACTIVE` | sex-pdx (shared) | SLOT_SILKBAR |
| `0xF4` | `OP_SILKBAR_FOCUS_STATE` | sex-pdx (shared) | SLOT_SILKBAR |
| `0xFC` | **FREE** ← candidate | — | — |
| `0xFD` | `OP_SURFACE_TAB_INFO` | sex-pdx (shared) | SLOT_DISPLAY |
| `0x202` | `OP_HID_EVENT` | silk-shell (local) | SLOT_SHELL |
| `0x260` | `OP_USB_MOUSE_REPORT` | silk-shell (local) | SLOT_SHELL |

### Opcode choice: `0xFC`

- `0xF5`–`0xFB` and `0xFC` are all free in the display opcode space
- `0xFC` is immediately adjacent to `0xFD` (existing OP_SURFACE_TAB_INFO)
- No collision with any existing sex-pdx or local constant
- No ABI_VERSION bump required — opcodes are not part of the ABI contract (they're protocol, not architecture)

### Opcode constant placement: sex-pdx shared constant

- **Decision: Add `OP_APPEARANCE_TOKENS: u64 = 0xFC` to `crates/sex-pdx/src/lib.rs`**
- Rationale: Both silk-shell (sender) and sexdisplay (receiver) reference this opcode. A shared constant in sex-pdx is the canonical pattern (see `OP_SURFACE_TAB_INFO` precedent at line 100).
- No ABI/contract update required — sex-pdx constants are not part of the build spec ABI guard (they're protocol constants, not architecture hash inputs).
- Build spec (`sexos_build_spec.toml`) does not register individual opcodes. No changes needed.

---

## RenderTokensV1 Struct

```rust
/// Render-safe appearance tokens sent from silk-shell to sexdisplay.
/// All colors are fully opaque (alpha = 0xFF) after clamping.
/// No alpha, no blur — flat ARGB only in V1.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RenderTokensV1 {
    /// Focused surface body fill color.
    pub focus_surface_color: u32,
    /// Neon rim color (left, right, bottom; top in minimal mode).
    pub frame_rim_color: u32,
    /// Top bar background color (default chrome mode).
    pub frame_top_bar_color: u32,
    /// Active tab block color.
    pub active_tab_color: u32,
    /// Inactive tab block color.
    pub inactive_tab_color: u32,
    /// Close light color (semantic red).
    pub close_light_color: u32,
    /// Minimize light color (semantic amber).
    pub minimize_light_color: u32,
    /// Zoom light color (semantic green).
    pub zoom_light_color: u32,
    /// Chrome/layout flags:
    ///   bit 0: reserved (must be 0 in V1; top bar controlled via 0xFD, not here)
    ///   bit 1..3: tab_strip_mode (0=always, 1=hover, 2=hidden) — reserved in V1
    ///   bit 4..6: frame_lights_mode (0=always, 1=hover, 2=hidden) — reserved in V1
    ///   bit 7: reserved
    pub chrome_flags: u8,
    /// Accessibility flags:
    ///   bit 0: high_contrast
    ///   bit 1: colorblind_safe
    ///   bit 2: stronger_focus_ring
    ///   bit 3: larger_targets
    ///   bit 4..7: reserved
    pub accessibility_flags: u8,
}
```

**Size:** `#[repr(C)]` natural alignment — 8 × u32 (32 bytes) + 2 × u8 (2 bytes) + 2 bytes padding = **36 bytes**.

### Default values (matching current hardcoded constants)

```rust
pub const DEFAULT_RENDER_TOKENS: RenderTokensV1 = RenderTokensV1 {
    focus_surface_color: 0x007AAFA4,
    frame_rim_color:     0x00B8F2E8,
    frame_top_bar_color: 0x0088C2B7,
    active_tab_color:    0x007AAFA4, // same as focus_surface_color (cascaded)
    inactive_tab_color:  0x006080B0,
    close_light_color:   0x00FF4444,
    minimize_light_color: 0x00FFCC44,
    zoom_light_color:    0x0044FF44,
    chrome_flags:        0b0000_0000, // top_bar is communicated via 0xFD, not here in V1
    accessibility_flags: 0b0000_0000,
};
```

### Clamping rules

| Rule | Enforcement | Rationale |
|------|-------------|-----------|
| Alpha must be 0xFF | `token \| 0xFF000000` | No transparency in V1. Forced opaque prevents invisible chrome. |
| Zero/black policy | Allowed if RGB ≠ 0. Pure black `0xFF000000` is valid (user choice). Pure zero `0x00000000` is clamped to `0xFF000000`. | Zero (transparent black) would make chrome invisible. Clamp alpha to 0xFF preserves intent as close to black as possible. |
| Chrome_flags range | All bits ignored/zeroed in V1. Top bar controlled via 0xFD exclusively. Must be 0x00 in V1. | Eliminates redundant control path. |
| Accessibility_flags range | All bits accepted. Unknown bits ignored. | Forward-compatible. |
| Glow_strength placeholder | Not present in V1. Reserved for future. | Deferred to effect engine. |
| Opacity/transparency placeholder | Not present in V1. Reserved for future. | Deferred to effect engine. |
| Blur_level placeholder | Not present in V1. Reserved for future. Must be forced to 0. | Deferred to effect engine. |

---

## IPC Protocol

### Opcode

```rust
// In crates/sex-pdx/src/lib.rs:
pub const OP_APPEARANCE_TOKENS: u64 = 0xFC;
```

### Payload packing (3 × u64 args, 2 calls)

The 34-byte token payload (8 × u32 + 2 × u8) requires two sequential `pdx_call()` invocations:

**Call 1 — Token colors (6 × u32 = 24 bytes, fills all 3 args)**

```
arg0: focus_surface_color       (bits 0..31)
      | frame_rim_color << 32   (bits 32..63)

arg1: frame_top_bar_color       (bits 0..31)
      | active_tab_color << 32  (bits 32..63)

arg2: inactive_tab_color        (bits 0..31)
      | close_light_color << 32 (bits 32..63)
```

**Call 2 — Remaining tokens + flags (2 × u32 + 2 × u8 = 10 bytes)**

```
arg0: minimize_light_color      (bits 0..31)
      | zoom_light_color << 32  (bits 32..63)

arg1: chrome_flags              (bits 0..7, as u64)
      | accessibility_flags << 8 (bits 8..15)
      | reserved (bits 16..63, zero)

arg2: 0 (reserved; sequence disambiguation done by receiver state machine, not this field)
```

### Sexdisplay handler pseudocode

```rust
0xFC => {
    // OP_APPEARANCE_TOKENS: two-call sequence.
    // Call 1: colors 0-5 (6 × u32 packed in 3 args)
    // Call 2: colors 6-7 + flags (2 × u32 + 2 × u8 packed in arg0, arg1)
    
    // Receive and store in token sequence buffer.
    // On second call, apply all tokens to DISPLAY_TOKENS.
    
    // Clamping:
    //   token.focus_surface_color |= 0xFF000000;  // force opaque
    //   token.frame_rim_color      |= 0xFF000000;
    //   ...
    
    // If fb_live: redraw_surface_area()
}
```

### Sexdisplay token storage

```rust
/// Global appearance tokens for all surfaces (not per-surface).
/// Initialized to DEFAULT_RENDER_TOKENS at compile time.
/// Updated by OP_APPEARANCE_TOKENS from silk-shell.
/// All values are clamped on receive — no invalid state.
static mut DISPLAY_TOKENS: RenderTokensV1 = DEFAULT_RENDER_TOKENS;
```

**Key decision: Global token table (not per-surface).** All surfaces share the same appearance palette. Per-surface override would add complexity with no V1 use case. Future per-scene tokens would replace the global table with a scene-scoped one.

### Token sequence housekeeping

Sexdisplay reception state machine. Call 1 stores 3 args (6 colors); Call 2 commits.

```rust
/// Temporary storage for two-call token sequence.
/// Call 1 fills args 0-2 (6 colors packed). Call 2 commits to DISPLAY_TOKENS.
static mut TOKEN_BUF_CALL1_RECEIVED: bool = false;
static mut TOKEN_BUF_ARG0: u64 = 0; // Call 1 arg0: focus_surface_color | frame_rim_color << 32
static mut TOKEN_BUF_ARG1: u64 = 0; // Call 1 arg1: frame_top_bar_color | active_tab_color << 32
static mut TOKEN_BUF_ARG2: u64 = 0; // Call 1 arg2: inactive_tab_color | close_light_color << 32
```

**Sequencing: pure state machine — no arg2 tagging**

Do NOT use arg2 values to distinguish Call 1 from Call 2. Color data in arg2 of Call 1 can collide with any magic constant. Use `TOKEN_BUF_CALL1_RECEIVED` as the sole disambiguator.

```rust
// Call 1: all 3 args carry colors (arg2 = color4 | color5<<32)
pdx_call(SLOT_DISPLAY, OP_APPEARANCE_TOKENS,
    pack2(color0, color1),   // arg0: focus_surface_color, frame_rim_color
    pack2(color2, color3),   // arg1: frame_top_bar_color, active_tab_color
    pack2(color4, color5));  // arg2: inactive_tab_color, close_light_color

// Call 2: arg2 = 0 (reserved, unused)
pdx_call(SLOT_DISPLAY, OP_APPEARANCE_TOKENS,
    pack2(color6, color7),        // arg0: minimize_light_color, zoom_light_color
    flags_byte | (acc_byte << 8), // arg1: chrome_flags (0), accessibility_flags
    0u64);                         // arg2: reserved
```

Sexdisplay state machine:
- 0xFC received + `TOKEN_BUF_CALL1_RECEIVED == false` → Call 1: store arg0/arg1/arg2 in TOKEN_BUF_ARG0/1/2, set flag true.
- 0xFC received + `TOKEN_BUF_CALL1_RECEIVED == true` → Call 2: store arg0/arg1, reconstruct all 8 tokens from buffers + new args, clamp all, write DISPLAY_TOKENS, set flag false, redraw.
- Call 2 with flag false (orphaned): discard safely.

---

## Sexdisplay token substitution

### Where tokens replace hardcoded constants

| Location | Current constant | Token field |
|----------|-----------------|-------------|
| `composite_pixel()` Pass 2, line 153 | `FRAME_TOP_BAR_COLOR` | `DISPLAY_TOKENS.frame_top_bar_color` |
| `composite_pixel()` Pass 2, line 166 | `TAB_ACTIVE_COLOR` | `DISPLAY_TOKENS.active_tab_color` |
| `composite_pixel()` Pass 2, line 168 | `TAB_INACTIVE_COLOR` | `DISPLAY_TOKENS.inactive_tab_color` |
| `composite_pixel()` Pass 2, line 177 | `FRAME_LIGHT_CLOSE_COLOR` | `DISPLAY_TOKENS.close_light_color` |
| `composite_pixel()` Pass 2, line 182 | `FRAME_LIGHT_MINIMIZE_COLOR` | `DISPLAY_TOKENS.minimize_light_color` |
| `composite_pixel()` Pass 2, line 187 | `FRAME_LIGHT_ZOOM_COLOR` | `DISPLAY_TOKENS.zoom_light_color` |
| `composite_pixel()` Pass 2, line 206 | `FRAME_LIGHT_CLOSE_COLOR` | `DISPLAY_TOKENS.close_light_color` |
| `composite_pixel()` Pass 2, line 212 | `FRAME_LIGHT_MINIMIZE_COLOR` | `DISPLAY_TOKENS.minimize_light_color` |
| `composite_pixel()` Pass 2, line 218 | `FRAME_LIGHT_ZOOM_COLOR` | `DISPLAY_TOKENS.zoom_light_color` |
| `composite_pixel()` Pass 2, line 231 | `TAB_ACTIVE_COLOR` | `DISPLAY_TOKENS.active_tab_color` |
| `composite_pixel()` Pass 2, line 233 | `TAB_INACTIVE_COLOR` | `DISPLAY_TOKENS.inactive_tab_color` |
| `composite_pixel()` Pass 2, line 236/239/244 | `FRAME_RIM_COLOR` | `DISPLAY_TOKENS.frame_rim_color` |
| `composite_pixel()` Pass 2, line 248 | `FOCUS_SURFACE_COLOR` | `DISPLAY_TOKENS.focus_surface_color` |

### Substitution mechanism

```rust
// In composite_pixel() Pass 2, replace:
//   c = FRAME_TOP_BAR_COLOR;
// with:
//   c = DISPLAY_TOKENS.frame_top_bar_color;

// The hardcoded constants are removed after substitution is verified.
// During migration, both coexist with a compile-time default match check.
```

**Design constraint:** Token lookup adds no dynamic dispatch. `DISPLAY_TOKENS` is a `static mut` struct — field access is a constant-time load. No per-pixel condition explosion. Performance impact: zero (same cost as loading a constant).

### Preservation of hardcoded constants as compile-time defaults

The current constants (`FOCUS_SURFACE_COLOR`, `FRAME_RIM_COLOR`, etc.) remain as compile-time defaults:

```rust
// Compile-time defaults (keep as fallback)
const FOCUS_SURFACE_COLOR: u32 = 0x007AAFA4;

// Runtime token table (initialized to match defaults)
static mut DISPLAY_TOKENS: RenderTokensV1 = DEFAULT_RENDER_TOKENS;
```

**Fallback policy:** If no `OP_APPEARANCE_TOKENS` message ever arrives, `DISPLAY_TOKENS` stays at `DEFAULT_RENDER_TOKENS` which matches the current hardcoded constants exactly. Zero behavioral change.

---

## Silk-shell send point

### When to send tokens

| Event | Send? | Notes |
|-------|-------|-------|
| Boot (after sexdisplay init) | ✅ | Send default tokens |
| Per-frame init | ❌ | Not needed — tokens are global |
| Top bar toggle | ❌ | Already handled by 0xFD chrome_flags |
| Token change (future settings) | ✅ | Push updated tokens |

### Send helper (pseudocode)

```rust
/// Push the current render tokens to sexdisplay.
/// Sends OP_APPEARANCE_TOKENS in two sequential pdx_call invocations.
unsafe fn push_render_tokens(tokens: &RenderTokensV1) {
    // Call 1: colors 0-5 (arg2 carries color4+color5; sequence inferred by receiver state)
    pdx_call(SLOT_DISPLAY, OP_APPEARANCE_TOKENS,
        pack_u32_pair(tokens.focus_surface_color, tokens.frame_rim_color),
        pack_u32_pair(tokens.frame_top_bar_color, tokens.active_tab_color),
        pack_u32_pair(tokens.inactive_tab_color, tokens.close_light_color),
    );
    
    // Call 2: colors 6-7 + flags; arg2=0 (reserved)
    pdx_call(SLOT_DISPLAY, OP_APPEARANCE_TOKENS,
        pack_u32_pair(tokens.minimize_light_color, tokens.zoom_light_color),
        (tokens.chrome_flags as u64) | ((tokens.accessibility_flags as u64) << 8),
        0u64, // reserved
    );
}
```

### Token source

In V1, the token set is a global static in silk-shell:

```rust
/// Current render tokens. Initialized to defaults.
/// Updated by future settings app or keyboard shortcuts.
static mut SHELL_RENDER_TOKENS: RenderTokensV1 = DEFAULT_RENDER_TOKENS;
```

Boot sequence:
1. silk-shell initializes `SHELL_RENDER_TOKENS` to defaults
2. sexdisplay initializes `DISPLAY_TOKENS` to defaults
3. silk-shell calls `push_render_tokens()` during init (after sexdisplay is ready)
4. Sexdisplay receives tokens, clamps, stores, redraws
5. Visual output is identical to before (same default values)

---

## Implementation file list

### Modified files

| File | Changes |
|------|---------|
| `crates/sex-pdx/src/lib.rs` | Add `pub const OP_APPEARANCE_TOKENS: u64 = 0xFC;` |
| `servers/sexdisplay/src/main.rs` | Add `RenderTokensV1` struct (or import from sex-pdx), `DISPLAY_TOKENS` static, `0xFC` handler with two-call sequence and clamping, token substitution in `composite_pixel()` Pass 2 |
| `servers/silk-shell/src/main.rs` | Add `push_render_tokens()` helper, boot send call, `SHELL_RENDER_TOKENS` static, `OP_APPEARANCE_TOKENS` import |

### Created files

| File | Role |
|------|------|
| `docs/handoff/SCENE_RENDER_TOKENS_PLAN_V1.md` | This document |
| `docs/handoff/SCENE_RENDER_TOKENS_V1.md` | Future implementation handoff |

### NOT modified

- `kernel/` — no kernel changes
- `servers/silkbar/` — SilkBar uses its own Theme model (separate)
- `crates/silkbar-model/` — independent of frame chrome tokens
- `servers/sexusb/` — untouched
- `servers/sexinput/` — untouched
- `sexos_build_spec.toml` — no ABI/contract change needed
- `Cargo.toml` files — no new dependencies

---

## Token vs Theme distinction

Important architectural boundary:

| System | Scope | Ownership | Storage |
|--------|-------|-----------|---------|
| **RenderTokensV1** | Frame chrome colors (rim, top bar, lights, tabs) | sexdisplay applies, silk-shell controls | `static mut DISPLAY_TOKENS` in sexdisplay |
| **Theme** (silkbar-model) | SilkBar panel colors (bar fill, glow, chips, text) | sexdisplay applies, silkbar-model defines | `DEFAULT_THEME` in silkbar-model, `SilkBar` struct carries usage |

**These are separate.** Frame chrome tokens are for window decoration. SilkBar theme is for the system bar. They have different color palettes and different update paths. In V1, SilkBar theme is compiled-in and not runtime-configurable. Frame chrome tokens are the first runtime-configurable color system.

---

## Token update safety guarantees

| Guarantee | Mechanism |
|-----------|-----------|
| No invisible chrome | Alpha forced to 0xFF (opaque) by clamp |
| No invalid state | All tokens clamped on receive before storage |
| No partial update | Two-call sequence committed atomically on second call |
| No repaint storm | Single `redraw_surface_area()` after commit |
| No hit-test change | Tokens are color-only; geometry unchanged |
| No allocation | Fixed-size structs, no heap |
| No sexdisplay crash | Range-validated, saturating arithmetic for edge cases |
| Default always valid | `DEFAULT_RENDER_TOKENS` matches current hardcoded values |

---

## STOP Conditions

| Condition | Stop? | Mitigation |
|-----------|-------|------------|
| Token storage requires heap | ❌ STOP | Global static only. No Vec, no Box. |
| Token IPC requires multi-word PDX ABI change | ❌ STOP | Use sequence of single pdx_call calls. Current ABI supports 3 args × u64. |
| Token count exceeds packing capacity | ❌ STOP | 8 × u32 + 2 × u8 = 34 bytes fits in 2 calls × 3 args. Future: add more calls. |
| Per-pixel condition explosion in composite_pixel | ❌ STOP | Token lookup is constant-time field access. Zero per-pixel overhead vs constants. |
| sexdisplay needs to read from silk-shell memory | ❌ STOP | All data pushed via pdx_call. No shared memory. |
| Token update requires framebuffer realloc | ❌ STOP | Colors only. Framebuffer dimensions unchanged. |
| Settings app needed for V1 | ✅ Conditional | V1 sends default tokens at boot. No settings app needed. User toggle via F4 uses existing 0xFD path. |
| Stale Call 1 after silk-shell crash | ✅ Accepted V1 limitation | If silk-shell dies after Call 1, `TOKEN_BUF_CALL1_RECEIVED` stays true. New Call 1 overwrites the stale buffer (safe). Orphaned stale Call 2 (from crashed run) is guarded — sexdisplay discards Call 2 when `TOKEN_BUF_CALL1_RECEIVED` is false. Risk: zero at boot (both sides initialize to same defaults). |

---

## Pass Criteria

- [x] Docs-only: no code changes
- [x] Verdict SCENE_RENDER_TOKENS_SAFE_WITH_NEW_OPCODE
- [x] Opcode collision audit complete (0xFC is free)
- [x] Token struct defined (8 × u32 + 2 × u8 = 34 bytes)
- [x] Default values match current hardcoded constants
- [x] Clamping rules specified (alpha forced to 0xFF, zero-black policy)
- [x] IPC packing designed (2 calls, sequence bits in arg2)
- [x] Sexdisplay storage designed (global static, initialized to defaults)
- [x] Substitution locations audited (15 sites in composite_pixel)
- [x] Fallback/zero-behavioral-change when no token message arrives
- [x] Shell send point identified (boot init)
- [x] Implementation file list complete
- [x] Forbidden files identified
- [x] STOP conditions documented
- [x] Token vs Theme distinction documented
- [x] Safety guarantees documented
- [x] Next phase named: SCENE_RENDER_TOKENS_V1

---

## Files

| File | Role |
|------|------|
| `docs/handoff/SCENE_RENDER_TOKENS_PLAN_V1.md` | This document |
| `.claude/plans/splendid-brewing-starlight.md` | Updated roadmap |

### References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_APPEARANCE_CONTROLS_PLAN_V1.md` | Parent taxonomy: Intent → Token → Pixel |
| `docs/handoff/FRAME_TOP_BAR_RENDER_V1.md` | Precedent for 0xFD extension (chrome_flags in arg2 bit 8) |
| `docs/handoff/FRAME_TOP_BAR_TOGGLE_V1.md` | Current top bar toggle mechanism |
| `docs/handoff/FRAME_GLASS_TINT_TUNE_V1.md` | Current tint scaffolding (replaced by tokens) |
| `docs/handoff/SILK_CHROME_SETTINGS_PLAN_V1.md` | Chrome mode settings roadmap |
| `docs/SILK_DE_GLASS_VISUAL_LANGUAGE.md` | Glass visual language, forbidden effects list |
| `crates/sex-pdx/src/lib.rs` | Opcode space audit (OP_SURFACE_TAB_INFO = 0xFD precedent) |
| `servers/sexdisplay/src/main.rs` | Color constants, composite_pixel substitution sites |
| `servers/silk-shell/src/main.rs` | send_frame_tab_info() pattern for IPC |
| `crates/silkbar-model/src/lib.rs` | Theme struct (separate from tokens) |

---

## Next Phase

### SCENE_RENDER_TOKENS_V1

Implement the RenderTokens protocol:

1. Add `OP_APPEARANCE_TOKENS = 0xFC` to `crates/sex-pdx/src/lib.rs`
2. Add `RenderTokensV1` struct and `DEFAULT_RENDER_TOKENS` (in sexdisplay or sex-pdx)
3. Add `DISPLAY_TOKENS` global static to sexdisplay
4. Add `0xFC` handler to sexdisplay main loop: two-call sequence, clamping, commit, redraw
5. Substitute token lookups in `composite_pixel()` Pass 2 (15 replacement sites)
6. Add `push_render_tokens()` helper to silk-shell
7. Add boot init call to push default tokens
8. Add `[shell.appearance.tokens.send]` and `[sexdisplay.appearance.tokens.receive]` markers
9. Build + verify: zero visual change at boot
10. Create `docs/handoff/SCENE_RENDER_TOKENS_V1.md`
