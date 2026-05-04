# PHASE 01: Silk Display Contract + Deterministic Rendering

## Goal
Explicit ABI/layout/theme contract between silkbar and sexdisplay. Startup validation catches mismatch before visual artifacts. Deterministic render verification via golden hash. Color-only SilkGlass polish (safe flat ARGB constant swaps only).

## Ownership
- **silkbar** (producer): contract constants, startup self-validation
- **sexdisplay** (consumer): contract validation on receive, render verification
- **silkbar-model** (shared): canonical contract layout (single source of truth)

## What Already Exists
- `silkbar-model` has `Theme` struct, `DEFAULT_THEME`, `SilkBar`, `UpdateKind`, `ModuleSlot`
- `sexdisplay` consumes `DEFAULT_THEME` for bar rendering
- `silkbar` producer exists but not yet connected via PDX (v7+)
- `OP_APPEARANCE_TOKENS` (0xFC) already implemented — color tokens flow shell→display
- Appearance token presets (BottleGlass, VioletGlass, GraphiteGlass, HighContrast) are live
- The glass color pipeline already works for frame chrome (Phase 1 is ~60% done)

## Bundle

| Task | File/Area | Detail | Effort |
|------|-----------|--------|--------|
| SILK_DE_BAR_ABI_V1 constant | `crates/silkbar-model/` | Canonical ABI version constant, layout hash | 1h |
| Contract validation (silkbar) | `servers/silkbar/src/main.rs` | Startup ABI self-check, reject mismatch | 2h |
| Contract validation (sexdisplay) | `servers/sexdisplay/src/main.rs` | Validate silkbar ABI on connect, reject mismatch | 2h |
| Top-strip deterministic render test | `servers/sexdisplay/src/main.rs` | Sample known-color pixels at strip zones, compare to golden | 3h |
| Golden hash/diff | `scripts/gate_render.sh` | Expected render output hash vs actual | 1h |
| Color-only SilkGlass polish | `servers/sexdisplay/src/main.rs` | Color constant swaps per glass doc — no alpha/blur/shadow | 1h |

## Smallest First Step
Ship the `SILK_DE_BAR_ABI_V1` constant and the ABI self-check in silkbar. That's one constant + one `if` statement at startup. It immediately catches version skew between silkbar and silkbar-model.

## Dependencies
- **Blocking**: None (independent of kernel, shell, input)
- **Blocked by**: Nothing
- **Can parallelize with**: Phase 0 (gates), Phase 2 (shell model) — separate ownership domains
- **Key insight**: This phase and Phase 2 don't touch the same files. The shell model work and the display contract work can happen simultaneously.

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Golden render hash is fragile (GPU/driver differences) | Medium | Medium | Hash specific pixel locations, not entire framebuffer. Use relative tolerances. |
| SilkBar not yet PDX-connected | High | Low (ABI check is async) | Contract validation is a passive check at startup. SilkBar connection is v7+, not this phase. |
| Color polish oversteps into alpha/blur | Low | High | Gate against alpha < 0xFF. Script `gate_no_alpha.sh` rejects any constant with alpha < 0xFF. |

## Exit Criteria (Done Checklist)
- [ ] `SILK_DE_BAR_ABI_V1` constant in silkbar-model
- [ ] silkbar validates ABI on startup (logs match or mismatch)
- [ ] sexdisplay validates ABI on silkbar connect (rejects mismatch)
- [ ] `scripts/gate_render.sh` samples known pixels and matches golden hash
- [ ] Color constants updated per bottle-glass palette (already done in FOCUS_SURFACE_COLOR etc.)
- [ ] `gate_no_alpha.sh` confirms all color constants have alpha = 0xFF
- [ ] Default build + boot passes
- [ ] No new warnings

## Testing Strategy
- **Unit**: ABI constant equality test in silkbar-model
- **Integration**: Boot QEMU, grep for ABI match log lines
- **Render**: Gate script samples specific pixel coordinates (e.g., center of SilkBar panel), compares to expected ARGB hex value
- **Regression**: Golden hash changes are intentional — require explicit update to golden file

## Efficiency Opportunity
**Merge the render verification with the gate infrastructure from Phase 0.** `scripts/gate_render.sh` follows the same pattern as `gate_boot.sh`. The gate runner `gate_all.sh` should include render verification. This cross-phase integration means Phase 1 produces a gate that Phase 0's runner consumes — two phases collaborate.

## Completeness Gain
Display/top strip: **35–45% → 55–65%**

## Files Changed
- `crates/silkbar-model/src/lib.rs` (+`SILK_DE_BAR_ABI_V1`)
- `servers/silkbar/src/main.rs` (startup ABI check)
- `servers/sexdisplay/src/main.rs` (consumer ABI check, render verification, color polish)
- `scripts/gate_render.sh` (new)
- `scripts/gate_no_alpha.sh` (new)

## Forbidden
- `kernel/` edits
- sex-pdx ABI changes (unless STOP FIRST)
- Alpha/blur/shadow (gate catches this)
- Layout/geometry changes
- Broad refactor

## Next Phase
PHASE_02_SHELL_SURFACE_OWNERSHIP_SCENE_FRAME_TAB.md

## Parallel Note
Phase 0 (gates) and Phase 1 (display contract) can proceed simultaneously. The gate runner from Phase 0 will later incorporate the render hash from Phase 1.
