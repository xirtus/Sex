# SILK_DE_CONTRACT_DRIFT_HARDEN_V1

Status: **COMPLETE** (2026-05-07)
Parent: SILK_DE_CONTRACT_AUDIT_V1
Zero behavioral change. Documentation + import consolidation + token index naming only.

## Files Changed

| File | Lines changed | Change type |
|------|--------------|-------------|
| `crates/silkbar-model/src/lib.rs` | ~15 | Added APPEARANCE_TOKEN_* constants (8+1), ABI doc markers, SetThemeToken GATE comment |
| `servers/silk-shell/src/main.rs` | ~25 | Import OPTION_*+SILKBAR_WORKSPACE_COUNT+APPEARANCE_TOKEN_* from model; removed local duplicates; named token indices in push_token_preset |
| `servers/sexdisplay/src/main.rs` | ~12 | Import APPEARANCE_TOKEN_*; BAR_H comment bridge; top strip boundary comments |

## Constants Consolidated

### OPTION_* (M1 resolved)

| Before | After |
|--------|-------|
| silk-shell defined local `OPTION_CLOSE=1, ZOOM=2, MINIMIZE=4, MOVE=8` at lines 4203-4209 | Imported from `silkbar-model`; local definitions removed |
| No compile-time link between shell and model | Single source of truth in model crate; Cargo.toml dep enforces consistency |

### WORKSPACE_COUNT (M2 resolved)

| Before | After |
|--------|-------|
| silk-shell: `const WORKSPACE_COUNT: u8 = 5;` | `const WORKSPACE_COUNT: u8 = SILKBAR_WORKSPACE_COUNT as u8;` |
| Independent value, type `u8` | Derived from model's `SILKBAR_WORKSPACE_COUNT`, cast to `u8` |

### Appearance Token Indices (M7 resolved)

| Before | After |
|--------|-------|
| Magic numbers `p[0]`..`p[7]` in push_token_preset | Named `p[APPEARANCE_TOKEN_FOCUS_SURFACE]` etc. |
| Implicit positional contract between shell and display | 8 `pub const APPEARANCE_TOKEN_*: usize` in silkbar-model; both sides import and reference |
| No way to detect reorder | Any reorder of constants would require coordinated change in model + both servers |

### ABI Version Documentation (M4 documented)

| Before | After |
|--------|-------|
| `ABI_VERSION=3` and `SILKBAR_ABI_VERSION=2` without clear distinction | Doc comments explain: model layout version vs PDX wire protocol version |
| `SILK_DE_BAR_ABI_V1` tautology checkout unexplained | Comment documents it must equal ABI_VERSION; checked by validate_contract |

### SetThemeToken No-Op (M8 documented)

| Before | After |
|--------|-------|
| Comment: "acknowledged but no-op" | GATE comment: explains tokens go via 0xFC to sexdisplay, this slot reserved for future |

### Bar Geometry Bridge (M3 documented)

| Before | After |
|--------|-------|
| `BAR_H = 50` unexplained | Comment: "covers model PANEL_Y+PANEL_H (10+38=48) plus 2px safety margin" |
| `y < 50` magic number | Comments at both render sites: "Top strip boundary: BAR_H = 50. Keep in sync" |

## Constants Left Duplicated (and Why)

| Constant | Reason not consolidated |
|----------|------------------------|
| Frame chrome constants (FRAME_RIM_PX, FRAME_LIGHT_SIZE_PX, etc.) | Require type conversion (i32 vs usize) across shell/display boundary. Shared ChromeTemplate would need design for type semantics. Deferred to SILK_DE_CONTRACT_SHARED_CHROME_V1. |
| Token slot count `8` in TokenPreset type | Used in type alias `type TokenPreset = [u32; 8]`. The `APPEARANCE_TOKEN_COUNT = 8` constant exists in model but can't replace the type-level `8` without const generics refactor. Comment bridge added. |
| SILK_CHROME_TEMPLATE_DEFAULT fields | Template struct with Rect fields for controls — display doesn't use Rect type. Full migration needs design. |

## Build Result

| Crate | `cargo check` | Notes |
|-------|--------------|-------|
| `silkbar-model` | PASS | Clean, no warnings |
| `silk-shell` | PASS | 493 pre-existing warnings (static mut), no new |
| `sexdisplay` | PASS | 3 pre-existing warnings, no new |
| `silkbar` | PASS (check) | Link fails on memcpy/memcmp — pre-existing infrastructure |

Full build with linking requires the project's custom pipeline (`./scripts/entrypoint_build.sh`)
which provides the OS-level memcpy/memcmp symbols.

## No Behavior Changes

All changes are:
- Import path changes (same values, different source)
- Doc comments
- Constant name substitutions (no value changes)

The following remain unchanged:
- No renderer policy ownership changes
- No framebuffer bounds changes
- No layout/color/animation changes
- No wire protocol changes
- No opcode changes
- No surface registry changes

## Next Prompt

```
HARDENED: SILK_DE_CONTRACT_DRIFT_HARDEN_V1 COMPLETE.

3 MODERATE risks resolved (M1, M7) or documented (M5 deferred).
4 LOW risks resolved (M2, M4) or documented (M3, M6 deferred).
3 INFO items documented (M8, M9, M10).

STATUS: Silk DE contract baseline is hardened.
Proceed with SilkBar/sexdisplay model contract work on this baseline.

NEXT: Planned SilkBar model contract work. All servers import
canonical constants from silkbar-model. No duplicate definitions
remain for OPTION_*, WORKSPACE_COUNT, or appearance token indices.
Frame chrome constants remain deferred to SILK_DE_CONTRACT_SHARED_CHROME_V1.
```
