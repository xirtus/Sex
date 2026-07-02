# ATLAS_SCENE_SETTINGS_MODEL_V1

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Extends the existing `Scene` struct with two new metadata fields — `accent` (u8)
and `pinned` (bool) — alongside helper functions for typed access with bounds
checking. All scenes are initialized with deterministic accent defaults cycling
through the 5 available tint bundles. No persistence, no UI, no sexdisplay changes.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Model extension + helpers + init |
| `docs/handoff/ATLAS_SCENE_SETTINGS_MODEL_V1.md` | New handoff doc |

---

## Model Changes

### `Scene` struct (line ~1410)

```rust
struct Scene {
    flags: u8,
    label: [u8; ATLAS_LABEL_LEN],
    /// Accent token: index into CUSTOM_TINT_BUNDLES (0..ACCENT_COUNT).
    /// 0 = Clear/default (no accent). Used to differentiate scene chrome.
    accent: u8,
    /// Pinned flag: when true, the scene survives frame-close operations
    /// and is not auto-destroyed when empty. Default false in V1.
    pinned: bool,
}
```

Both fields are additive — no existing code reads or writes them yet. The
`accent` token will be consumed by the future UI layer (card tint color in
Atlas, chrome accent in active scene). The `pinned` flag is reserved for
future scene-locking semantics.

### Accent Token Constants

```rust
const ACCENT_DEFAULT: u8 = 0;  // Clear (no accent)
const ACCENT_WARM: u8    = 1;  // WarmTint — amber/copper
const ACCENT_COOL: u8    = 2;  // CoolTint — icy blue
const ACCENT_CORAL: u8   = 3;  // CoralTint — pink/coral
const ACCENT_GOLD: u8    = 4;  // GoldTint — gold
const ACCENT_COUNT: u8   = 5;  // Matches CUSTOM_TINT_BUNDLES count
```

These correspond directly to `CUSTOM_TINT_BUNDLES` indices in the existing
theme system (lines 318-328).

### `SCENES` Static Initializer

Updated to include `accent: 0, pinned: false` defaults.

---

## Helpers Added

All helpers are `unsafe` (access `static mut SCENES`) with inline bounds
checking:

| Function | Signature | Returns | Marker |
|----------|-----------|---------|--------|
| `validate_scene_id` | `(scene_id: u8) -> bool` | `true` if in range | — |
| `scene_accent_token` | `(scene_id: u8) -> u8` | Accent index or `ACCENT_DEFAULT` | `[atlas.scene.settings.reject]` on invalid |
| `scene_is_pinned` | `(scene_id: u8) -> bool` | `true` if pinned | `[atlas.scene.settings.read]` + `[atlas.scene.settings.reject]` |
| `scene_label_token` | `(scene_id: u8) -> [u8; ATLAS_LABEL_LEN]` | Copy of label bytes | `[atlas.scene.settings.reject]` on invalid |

**Location:** After `scene_update_flags()`, before `atlas_capture_snapshot()`.

---

## Initialization

In `scene_init_all()`:

```rust
let default_accents: [u8; ATLAS_MAX_SCENES] = [
    ACCENT_DEFAULT, // Scene 0: Clear (no accent)
    ACCENT_WARM,    // Scene 1: Warm amber/copper
    ACCENT_COOL,    // Scene 2: Cool icy blue
    ACCENT_CORAL,   // Scene 3: Coral pink
    ACCENT_GOLD,    // Scene 4: Gold
];

for si in 0..ATLAS_MAX_SCENES {
    SCENES[si] = Scene {
        flags: SCENE_FLAG_EMPTY,
        label: atlas_default_label(si as u32),
        accent: default_accents[si],
        pinned: false,
    };
}
```

Each scene gets a distinct accent by default for visual differentiation.

---

## Proof Markers

| Marker | Budget | Location | Condition |
|--------|--------|----------|-----------|
| `[atlas.scene.settings.init]` | 4 | `scene_init_all()` | After all scenes initialized, prints accent array |
| `[atlas.scene.settings.read]` | 32 | `scene_is_pinned()` | On successful read of pinned flag |
| `[atlas.scene.settings.reject]` | 8 (shared) | `scene_accent_token()`, `scene_is_pinned()`, `scene_label_token()` | Invalid scene_id passed |

The `[atlas.scene.settings.reject]` budgets are per-function via separate
`static mut` counters (each initialized to 8).

---

## Deferred (Not Part of This Phase)

| Item | Reason |
|------|--------|
| UI rendering of accent token | Part of ATLAS_SCENE_SETTINGS_UI_V1 |
| Persistence of scene settings | No store protocol yet |
| Scene pinning semantics | No close-guard code path yet |
| User-settable labels | Requires text protocol |
| Scene-specific appearance overrides | Would need per-scene `SceneAppearanceState` |

---

## Build Verification

```sh
./scripts/entrypoint_build.sh
# Result: ISO produced, no errors
# Warnings: only pre-existing (unused import in sexstore, etc.)
```

---

## References

- `CUSTOM_TINT_BUNDLES` — lines 318-328, indexed by accent token
- `Scene` struct — line ~1410, now has accent + pinned fields
- `scene_init_all()` — line ~2175, accent initialization
- `ACTIVE_TINT_IDX` — global u8 at line ~331, separate from per-scene accent
