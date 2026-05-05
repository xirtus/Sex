# ATLAS_SCENE_SETTINGS_VISUALS_V1

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Renders existing Scene settings metadata (accent token, pinned flag) in the Atlas overview. Small visual patch to `servers/silk-shell/src/main.rs` only. No sexdisplay protocol changes, no sex-pdx edits, no storage touches.

### Visual behavior

| Setting | Visual | When |
|---------|--------|------|
| **Accent** | Card background fill uses accent color from `ATLAS_ACCENT_COLORS` (matching `CUSTOM_TINT_BUNDLES` rim colors, dimmed for card context) | Non-empty scene with non-zero accent token, not navigation-selected |
| **Pinned** | Small 8×8 gold dot (`ATLAS_PIN_COLOR = 0x00FFDD44`) at top-right corner of card | Scene has `pinned == true` |
| **Label** | Deferred — `SceneDescriptor` already carries label bytes but rendering text requires sexdisplay text ops or font blitting (out of scope) | — |

### Examples

```
Scene 0: accent=1 (Warm), pinned=false → amber-tinted card, no dot
Scene 2: accent=3 (Coral), pinned=true  → coral-tinted card, gold dot at top-right
Scene 4: accent=0 (Clear), pinned=false → default card color, no dot
```

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | ~50 lines (SceneDescriptor + constants + snapshot + render + proof markers) |
| `docs/handoff/ATLAS_SCENE_SETTINGS_VISUALS_V1.md` | New handoff doc |

---

## Changes Detail

### 1. SceneDescriptor — new metadata fields

```rust
struct SceneDescriptor {
    scene_id: u32,
    label: [u8; ATLAS_LABEL_LEN],
    flags: u8,
    accent: u8,        // NEW: accent token index (0..ACCENT_COUNT)
    pinned: bool,      // NEW: pinned flag
    focused_frame_id: u32,
    frame_count: u8,
    frame_ids: [u32; ATLAS_MAX_FRAMES_PER_SCENE],
}
```

### 2. New constants

```rust
/// Atlas accent card colors (ARGB). Maps ACCENT_DEFAULT..ACCENT_GOLD.
const ATLAS_ACCENT_COLORS: [u32; ACCENT_COUNT as usize] = [
    0x00000000, // 0: Clear (use default card color)
    0x00805020, // 1: Warm — amber/dark copper
    0x00205080, // 2: Cool — muted icy blue
    0x00804050, // 3: Coral — muted pink
    0x00807000, // 4: Gold — muted gold
];

/// Color of the pinned indicator dot (bright gold).
const ATLAS_PIN_COLOR: u32 = 0x00FFDD44;
```

These are derived from the rim colors in `CUSTOM_TINT_BUNDLES` (lines 1385-1396), dimmed ~40% for card background use. The mapping is conceptual — accent token N corresponds to CUSTOM_TINT_BUNDLES[N]'s rim color.

### 3. Snapshot population (atlas_capture_snapshot)

```rust
sd.accent = SCENES[scene_idx].accent;
sd.pinned = SCENES[scene_idx].pinned;
```

Added after `sd.label = SCENES[scene_idx].label;` in the scene iteration loop.

### 4. Card color selection (atlas_render_stub)

Modified the card color decision tree:

```
selected          → ATLAS_CARD_SELECTED_COLOR (unchanged)
empty             → ATLAS_CARD_EMPTY_COLOR (unchanged)
accent != 0 && valid → ATLAS_ACCENT_COLORS[accent] ← NEW
accent != 0 && OOB   → reject marker, fall through to active/default ← NEW
active            → ATLAS_CARD_ACTIVE_COLOR (unchanged)
default           → ATLAS_CARD_COLOR (unchanged)
```

### 5. Pinned indicator

After frame block rendering, if `sd.pinned` is true:

```rust
let dot_size: i32 = 8;
let dot_x = cx + card_w as i32 - dot_size - 4;
let dot_y = cy + 4;
pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
    (dot_y as u64) << 32 | dot_x as u64,
    (ATLAS_PIN_COLOR as u64) << 32 | (dot_size as u64) << 16 | dot_size as u64);
```

Position: 4px from top, 4px from right edge of card. No overlap with card border (border is drawn after pinned dot, so dot is beneath border — invisible when selected but visible in overview).

### 6. Label

Deferred. `SceneDescriptor.label` is already populated from `SCENES[scene_idx].label` in `atlas_capture_snapshot()`. Rendering label text in Atlas cards would require either:
- A sexdisplay text rendering op (new protocol — out of scope)
- Font blitting in silk-shell (complex — out of scope)

---

## Shell/Display Ownership Boundary

| Responsibility | Owner | Verification |
|---------------|-------|-------------|
| Scene model (accent, pinned) | silk-shell (Scene struct) | ✅ Existing, no change |
| Scene snapshot rendering | silk-shell (atlas_render_stub) | ✅ Modified — reads accent/pinned from SceneDescriptor |
| Surface rendering | sexdisplay (fill rect via 0xEF) | ✅ Unchanged — sexdisplay is renderer-only |
| Scene policy/lifecycle | silk-shell | ✅ Unchanged |
| Storage/persistence | sexstore | ✅ Not touched |

sexdisplay remains **renderer-only**: it receives only `0xEC` (surface create) and `0xEF` (fill rect) calls. No scene policy, no lifecycle logic, no storage reads, no semantic ownership.

---

## Proof Markers

| Marker | Budget | Location | When |
|--------|--------|----------|------|
| `[atlas.scene.visual.accent]` | 8 | `atlas_render_stub()` card color block | Scene rendered with non-zero accent color |
| `[atlas.scene.visual.pinned]` | 8 | `atlas_render_stub()` pinned dot block | Scene rendered with pinned indicator |
| `[atlas.scene.visual.reject]` | 8 | `atlas_render_stub()` card color else block | Accent token non-zero but out of bounds |

All markers are StructuralMeta (E8 class) — no stored values, no content, no paths.

---

## Build Verification

```sh
./scripts/entrypoint_build.sh
# Result: ISO produced, no errors
```

---

## STOP FIRST Findings

| # | Condition | Check | Stop? |
|---|-----------|-------|-------|
| S1 | Atlas/display payload needs ABI change | `SceneDescriptor` is silk-shell internal. sexdisplay untouched. No PDX change. | ❌ Not triggered |
| S2 | Rendering needs sexdisplay to own Scene policy | sexdisplay receives only `0xEC`/`0xEF` calls. All scene logic in silk-shell. | ❌ Not triggered |
| S3 | Bounds checks weakened | Card positions via `atlas_card_pos()` unchanged. Accent bounds checked via `(sd.accent as usize) < ACCENT_COUNT as usize`. Pinned is bool. | ❌ Not triggered |
| S4 | Storage or persistence involved | No sexstore/sexshop/sex-pdx touched. | ❌ Not triggered |
| S5 | Label rendering requires strings/heap/content | Label rendering deferred. No strings, no heap, no content logging. | ❌ Not triggered |

---

## Diff Summary

```
SceneDescriptor:           +2 fields (accent, pinned)     struct definition
                           +2 fields (accent, pinned)     initializer (static ATLAS_SNAPSHOT)
                           +2 fields (accent, pinned)     initializer (atlas_capture_snapshot)
atlas_capture_snapshot():  +2 lines (sd.accent, sd.pinned)
                           +2 fields (accent, pinned)     init struct
atlas_render_stub():       +15 lines (accent color override)
                           +5 lines (accent proof marker)
                           +10 lines (reject marker)
                           +10 lines (pinned dot + marker)
Constants:                 +6 lines (ATLAS_ACCENT_COLORS[5])
                           +1 line  (ATLAS_PIN_COLOR)
                           +1 line  (ATLAS_CARD_INACTIVE_RIM_COLOR moved)

Total: ~50 lines added, 0 removed
```

---

## References

- `ATLAS_SCENE_SETTINGS_MODEL_V1.md` — scene_accent_token(), scene_is_pinned(), ACCENT_COUNT
- `ATLAS_SCENE_SETTINGS_UI_V1.md` — keyboard controls for accent/pin mutation
- `CUSTOM_TINT_BUNDLES` (line ~1385) — tint bundles mapped by accent token
- `atlas_render_stub()` (line ~4485) — main render function
- `atlas_capture_snapshot()` (line ~4107) — snapshot derivation
- `SceneDescriptor` (line ~3167) — descriptor struct
- `Scene` (line ~3184) — runtime scene state

---

*End of ATLAS_SCENE_SETTINGS_VISUALS_V1.md*