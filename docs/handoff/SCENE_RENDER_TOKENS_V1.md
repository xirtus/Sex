# SCENE_RENDER_TOKENS_V1

## Status

Complete (2026-05-04). Renderer-safe flat ARGB scene render tokens implemented and verified via `./scripts/entrypoint_build.sh` [SEXOS ENTRYPOINT] success.

---

## Changed Files

| File | Change |
|------|--------|
| `crates/sex-pdx/src/lib.rs` | Added `OP_APPEARANCE_TOKENS: u64 = 0xFC` |
| `servers/sexdisplay/src/main.rs` | Added `RenderTokensV1`, `DEFAULT_RENDER_TOKENS`, `DISPLAY_TOKENS`, token reception state machine, `0xFC` handler, 13 composite_pixel substitutions |
| `servers/silk-shell/src/main.rs` | Added `OP_APPEARANCE_TOKENS` import, 8 `DTOK_*` constants, `pack_u32_pair()`, `send_scene_render_tokens()`, boot send call |
| `sexos_build_spec.toml` | Updated `abi_version_hash` (sex-pdx change triggers recompute) |

### NOT modified

- `kernel/` — no kernel changes
- `servers/silkbar/` — SilkBar theme is independent
- `crates/silkbar-model/` — independent
- `servers/sexusb/` — untouched
- `servers/sexinput/` — untouched
- `sexos_contract.toml` — no contract change

---

## Opcode / Payload Layout

**Opcode:** `OP_APPEARANCE_TOKENS = 0xFC` (in `crates/sex-pdx/src/lib.rs`)

**Protocol:** two sequential `pdx_call(SLOT_DISPLAY, OP_APPEARANCE_TOKENS, ...)` messages.

| Call | arg0 | arg1 | arg2 |
|------|------|------|------|
| 1 | `focus_surface_color \| (frame_rim_color << 32)` | `frame_top_bar_color \| (active_tab_color << 32)` | `inactive_tab_color \| (close_light_color << 32)` |
| 2 | `minimize_light_color \| (zoom_light_color << 32)` | `appearance_flags \| (effect_levels << 8)` | `0` (reserved) |

**Sequencing:** pure state machine in sexdisplay. `TOKEN_BUF_CALL1_RECEIVED: bool` distinguishes calls. No arg2 tagging (arg2 of Call 1 carries color data; any bit-based tag would alias color values).

---

## Token Fields / Defaults / Clamps

### Struct (`servers/sexdisplay/src/main.rs`)

```rust
struct RenderTokensV1 {
    focus_surface_color:  u32,  // 0x007AAFA4
    frame_rim_color:      u32,  // 0x00B8F2E8
    frame_top_bar_color:  u32,  // 0x0088C2B7
    active_tab_color:     u32,  // 0x007AAFA4
    inactive_tab_color:   u32,  // 0x006080B0
    close_light_color:    u32,  // 0x00FF4444
    minimize_light_color: u32,  // 0x00FFCC44
    zoom_light_color:     u32,  // 0x0044FF44
    appearance_flags:     u8,   // 0x00 (all reserved in V1)
    effect_levels:        u8,   // 0x00 (blur=0 forced, low nibble zeroed)
}
```

### Clamping (applied on receive in 0xFC handler)

- All colors: `c | 0xFF000000` — forces alpha byte to 0xFF. Renderer ignores alpha (raw framebuffer write), but this guards future alpha paths.
- `effect_levels`: low nibble zeroed (`& !0x0F`) — blur forced to 0 in V1.
- `appearance_flags`: stored as-is; all bits reserved in V1 (sexdisplay ignores them).

### Zero behavioral change guarantee

`DEFAULT_RENDER_TOKENS` matches current hardcoded constants exactly. If silk-shell boot send is skipped, `DISPLAY_TOKENS` stays at defaults. Visual output is identical.

---

## Render Substitution Summary

13 substitution sites in `composite_pixel()` Pass 2:

| Location | Was | Now |
|----------|-----|-----|
| Top bar background | `FRAME_TOP_BAR_COLOR` | `DISPLAY_TOKENS.frame_top_bar_color` |
| Top bar active tab | `TAB_ACTIVE_COLOR` | `DISPLAY_TOKENS.active_tab_color` |
| Top bar inactive tab | `TAB_INACTIVE_COLOR` | `DISPLAY_TOKENS.inactive_tab_color` |
| Top bar close light | `FRAME_LIGHT_CLOSE_COLOR` | `DISPLAY_TOKENS.close_light_color` |
| Top bar minimize light | `FRAME_LIGHT_MINIMIZE_COLOR` | `DISPLAY_TOKENS.minimize_light_color` |
| Top bar zoom light | `FRAME_LIGHT_ZOOM_COLOR` | `DISPLAY_TOKENS.zoom_light_color` |
| Minimal mode close light | `FRAME_LIGHT_CLOSE_COLOR` | `DISPLAY_TOKENS.close_light_color` |
| Minimal mode minimize light | `FRAME_LIGHT_MINIMIZE_COLOR` | `DISPLAY_TOKENS.minimize_light_color` |
| Minimal mode zoom light | `FRAME_LIGHT_ZOOM_COLOR` | `DISPLAY_TOKENS.zoom_light_color` |
| Minimal mode active tab | `TAB_ACTIVE_COLOR` | `DISPLAY_TOKENS.active_tab_color` |
| Minimal mode inactive tab | `TAB_INACTIVE_COLOR` | `DISPLAY_TOKENS.inactive_tab_color` |
| Rim (3 sites) | `FRAME_RIM_COLOR` | `DISPLAY_TOKENS.frame_rim_color` |
| Surface content fill | `FOCUS_SURFACE_COLOR` | `DISPLAY_TOKENS.focus_surface_color` |

All sites are inside the existing `unsafe {}` block in `composite_pixel()`. No render order or bounds check changes.

Old constants (`FOCUS_SURFACE_COLOR`, `FRAME_RIM_COLOR`, etc.) kept as compile-time values and referenced by `DEFAULT_RENDER_TOKENS` — no dead_code warnings.

---

## Build / Proof Result

```
[SEXOS ENTRYPOINT] success
```

- All ABI/pipeline guards pass
- `abi_version_hash` updated in `sexos_build_spec.toml` (sex-pdx change required recompute)
- `sexos_contract.toml` unchanged

### Log markers (boot run)

```
[shell.appearance.tokens.send] seq=2 sent       ← silk-shell sent both calls
[sexdisplay.appearance.tokens] seq=0 buffered   ← display received Call 1
[sexdisplay.appearance.tokens] seq=1 applied=1  ← display received Call 2, committed
```

---

## Limitations

- **Flat colors only** — no alpha blending, no compositing. Renderer writes raw `u32` to framebuffer; alpha byte ignored by hardware.
- **No real blur** — `effect_levels` low nibble zeroed on receive. No blur pipeline exists.
- **No transparency** — `appearance_flags` reserved; no opacity token in V1.
- **No settings app** — tokens are pushed at boot with hardcoded defaults. Runtime color changes require a future settings path.
- **Global token table** — all surfaces share one `DISPLAY_TOKENS`. No per-surface or per-scene override in V1.
- **No persistence** — tokens reset to defaults on reboot (they're compile-time statics).

---

## Next Recommended Phase: SCENE_RENDER_TOKENS_TOGGLE_COLOR_V1

Add a keyboard shortcut (e.g. F5) in silk-shell that cycles through a small palette of preset token sets and pushes updated tokens via `send_scene_render_tokens()`.

Steps:
1. Define 2-3 preset `[u32; 8]` color arrays in silk-shell
2. Add a `CURRENT_PALETTE_IDX` static
3. Bind F5 to `cycle_scene_palette()` → updates DTOK_* values, calls `send_scene_render_tokens()`
4. No sexdisplay or sex-pdx changes needed
5. Verify: F5 changes chrome colors at runtime; F4 top bar toggle still works; no panic

This proves the token pipe is live before adding a full settings app.
