# SILK_DE_CONTRACT_AUDIT_V1

Status: **AUDIT COMPLETE** (2026-05-07)
Scope: Top-strip model/render contract drift audit before SilkBar/sexdisplay model contract work.
No code changes — documentation, mismatch list, risk table, invariant list, and smallest patch prompt only.

## Files Audited

| File | Role |
|------|------|
| `crates/silkbar-model/src/lib.rs` | Shared model: types, constants, ABI, update logic (755 lines) |
| `servers/silkbar/src/main.rs` | Producer: pushes SilkBarUpdate to sexdisplay (311 lines) |
| `servers/sexdisplay/src/main.rs` | Consumer/sole framebuffer writer: render + surface registry (1562 lines) |
| `servers/silk-shell/src/main.rs` | Shell policy owner: workspace switch, focus, chrome tokens (sampled ~12k lines) |

## 1. Mismatch List

### M1: OPTION_* Duplication (MODERATE)

silk-shell defines `OPTION_CLOSE/ZOOM/MINIMIZE/MOVE` at lines 4203-4209 independently of `silkbar-model`.
silk-shell depends on silkbar-model but imports only `DEFAULT_SILK_BAR, hit_test_action, Action, PANEL_X, PANEL_Y, PANEL_W, PANEL_H`.

| Constant | silkbar-model | silk-shell | Match? |
|----------|--------------|------------|--------|
| OPTION_CLOSE | `1` (line 18) | `1` (line 4203) | values match |
| OPTION_ZOOM | `2` (line 20) | `2` (line 4205) | values match |
| OPTION_MINIMIZE | `4` (line 22) | `4` (line 4207) | values match |
| OPTION_MOVE | `8` (line 24) | `8` (line 4209) | values match |

**Risk**: If silkbar-model changes option bit assignments, silk-shell silently uses stale values.
No compile-time enforcement — Cargo.toml dep exists but the symbols are not imported.

**Drift vector**: silk-shell's `selected_window_options_mask()` computes a mask from
local constant values. If silkbar-model renumbers bits, sexdisplay's `bar_color()` option-dot
rendering (lines 403-428) will render wrong colors for wrong bits.

### M2: WORKSPACE_COUNT Duplication (LOW)

| File | Constant | Value | Type |
|------|---------|-------|------|
| silkbar-model | `WORKSPACE_COUNT` | `5` | `usize` |
| silkbar-model | `SILKBAR_WORKSPACE_COUNT` | `WORKSPACE_COUNT` | `usize` |
| silk-shell | `WORKSPACE_COUNT` | `5` | `u8` |

**Risk**: If WORKSPACE_COUNT changes in one place but not the other, scene switching
and SilkBar workspace indicator rendering desync. Silkbar silently clamps
via `.min(SILKBAR_WORKSPACE_IDX_MAX)`, so the failure is silent corruption, not a crash.

### M3: Bar Height Hardcoding vs Model Geometry (LOW)

sexdisplay hardcodes `y < 50` and `BAR_H = 50` for the top-strip boundary.
Model defines `PANEL_Y=10, PANEL_H=38` -> panel occupies rows y=10..47.

| Parameter | Model Value | Renderer Value | Delta |
|-----------|-------------|----------------|-------|
| Panel bottom (exclusive) | `PANEL_Y + PANEL_H = 48` | `50` | +2px |
| Surface clamp minimum y | N/A in model | `BAR_H = 50` | — |

**Effect**: Rows 48-49 are rendered as `DEFAULT_THEME.panel_fill` (bar background) despite
being outside the model-defined panel geometry. Row 50 is the glow edge. Surfaces clamped
to y >= 50, leaving a 2px dead band.

**Safety**: Conservative (protects more space than model requires). If the
model's panel geometry changes, this hardcoding could become too narrow or too wide.

### M4: ABI_VERSION Dual Semantics (LOW)

Model tracks two version numbers with different meanings:

| Constant | Value | Semantics | Checked by |
|----------|-------|-----------|------------|
| `ABI_VERSION` | `3` | Model layout + UpdateKind version | `validate_contract()` self-check |
| `SILK_DE_BAR_ABI_V1` | `3` | Must equal ABI_VERSION | `validate_contract()` equality check |
| `SILKBAR_ABI_VERSION` | `2` | PDX wire protocol version | Returned by `OP_SILKBAR_GET_ABI` |
| `SILKBAR_UPDATE_SIZE` | `16` | Wire struct size | Compile-time assert |

The `validate_contract()` check `ABI_VERSION != SILK_DE_BAR_ABI_V1` is a tautology
(both always equal 3 since defined adjacently). `SILKBAR_ABI_VERSION=2` is separately
tracked for PDX compatibility — consumers query this value.

**Risk**: Developer might bump `ABI_VERSION` without realizing `SILKBAR_ABI_VERSION`
is the value consumers actually query at runtime.

### M5: Frame Chrome Constants Duplicated, Different Types (MODERATE)

| Parameter | silk-shell (ChromeTemplate) | sexdisplay (hardcoded) | Type Diff |
|-----------|----------------------------|------------------------|-----------|
| rim_px | `4` (line 3436) | `FRAME_RIM_PX = 4` (line 72) | i32 vs usize |
| top_bar_height_px | `16` (line 3437) | `FRAME_TOP_BAR_HEIGHT_PX = 16` (line 91) | i32 vs usize |
| light_size_px | `4` (line 3438) | `FRAME_LIGHT_SIZE_PX = 4` (line 76) | i32 vs usize |
| light_gap_px | `2` (line 3439) | `FRAME_LIGHT_GAP_PX = 2` (line 77) | i32 vs usize |
| top_bar_light_size_px | `8` (line 3440) | `FRAME_TOP_BAR_LIGHT_SIZE_PX = 8` (line 93) | i32 vs usize |
| top_bar_light_gap_px | `4` (line 3441) | `FRAME_TOP_BAR_LIGHT_GAP_PX = 4` (line 95) | i32 vs usize |
| top_bar_light_exclusion_px | `40` (line 3442) | `FRAME_TOP_BAR_LIGHT_EXCLUSION_PX = 40` (line 97) | i32 vs usize |
| tab_light_exclusion_px | `20` (line 3443) | `TAB_STRIP_LIGHT_EXCLUSION_PX = 20` (line 83) | i32 vs usize |
| tab_strip_px | `4` (line 3445) | _(not present in sexdisplay)_ | — |

**Status**: All values currently match. No shared source of truth. The i32 vs usize
type difference means silk-shell computations can produce negative values that
sexdisplay would interpret as large positive numbers (wrap-around). Currently safe
because no negative chrome coordinates are generated.

**Drift vector**: If silk-shell changes `SILK_CHROME_TEMPLATE_DEFAULT`, sexdisplay's
hardcoded constants will not follow. Frame light hit-test in silk-shell disagrees
with frame light rendering in sexdisplay.

### M6: focus_state Semantic Drift — Dead Paths (LOW)

Silkbar maps focus_state to workspace urgency at lines 198-210:
```
focus_state=0 -> no urgent
focus_state=1 -> ws0 urgent
focus_state=2 -> ws1 urgent   <- UNREACHABLE
focus_state=3 -> ws2 urgent   <- UNREACHABLE
```

Silk-shell only sends `OP_SILKBAR_FOCUS_STATE(0, 0)` or `OP_SILKBAR_FOCUS_STATE(1, mask)`.
Focus states 2 (app) and 3 (debug) are never sent. The workspace urgency mapping
is conflated with focus type — focus_state=1 always makes ws0 urgent regardless
of which scene is active. This is a dead feature.

### M7: Appearance Token Positional Contract (MODERATE)

Silk-shell `TokenPreset` is `[u32; 8]` with implicit index semantics:

| Index | Sexdisplay Field |
|-------|-----------------|
| 0 | `DISPLAY_TOKENS.focus_surface_color` |
| 1 | `DISPLAY_TOKENS.frame_rim_color` |
| 2 | `DISPLAY_TOKENS.frame_top_bar_color` |
| 3 | `DISPLAY_TOKENS.active_tab_color` |
| 4 | `DISPLAY_TOKENS.inactive_tab_color` |
| 5 | `DISPLAY_TOKENS.close_light_color` |
| 6 | `DISPLAY_TOKENS.minimize_light_color` |
| 7 | `DISPLAY_TOKENS.zoom_light_color` |

No enum, no named constants, no compile-time ordering enforcement. If either side
reorders, colors silently swap. Two-call state machine (0xFC) has no version tag.

### M8: SetThemeToken Always Returns false (INFO)

`apply_update` kind=5 (`SetThemeToken`) unconditionally returns `false` (line 429).
Comment says "Future: route to mutable theme storage." Known stub, no explicit marker.
Producers see silent drop — indistinguishable from invalid kind.

### M9: ChipSlot Discriminant Check is Tautological (INFO)

`validate_contract()` lines 642-645 check `ChipSlot::Chip0 as usize != 0` etc.
Always true because enum has explicit discriminants `Chip0=0, Chip1=1, Chip2=2, Clock=3`.
Serves as documentation. Real safety comes from `apply_update()` bounds checking
against `MAX_CHIPS`.

### M10: BellState _pad Never Modified (INFO)

`BellState._pad: u8` initialized to 0 via DEFAULT_SILK_BAR. `SetBellPresence` update
(kind=7) sets `total_visible`, `redacted_count`, `flags` but never touches `_pad`.
Harmless but if BellState gains real fields that alias the padding byte, existing
binary layouts break silently.

## 2. Risk Table

| ID | Risk | Severity | Likelihood | Impact | Drift Type |
|----|------|----------|------------|--------|------------|
| M1 | OPTION_* duplication | **MODERATE** | Low (values stable) | Bitmask semantic drift -> wrong option dots rendered | Independent constants |
| M2 | WORKSPACE_COUNT dup | **LOW** | Very Low | Scene switching silently clamped -> wrong indicators | Independent constants |
| M3 | Bar height hardcoding | **LOW** | Low | 2px dead band, surface clamp offset | Renderer vs model geometry |
| M4 | ABI_VERSION dual semantics | **LOW** | Low | PDX version negotiation stale | Two version numbers, one check |
| M5 | Chrome constants dup+type | **MODERATE** | Medium (planned work) | Light positions diverge hit-test vs render | No shared source of truth |
| M6 | focus_state dead paths | **LOW** | Very Low | Dead code, semantic conflation | Unwired feature |
| M7 | Token positional contract | **MODERATE** | Medium | Color slots swap -> visual corruption | Implicit index ordering |
| M8 | SetThemeToken no-op | **INFO** | N/A | Silent drop of valid opcode | Acknowledged stub |
| M9 | ChipSlot tautology | **INFO** | Zero | None — always passes | Self-referential check |
| M10 | BellState padding | **INFO** | Very Low | Future field collision | Padding byte unmanaged |

### Risk Summary

- **No CRITICAL or HIGH severity findings.**
- **3 MODERATE** risks: M1 (OPTION_* dup), M5 (chrome constants dup), M7 (token positional contract)
- **4 LOW** risks: M2, M3, M4, M6
- **3 INFO**: M8, M9, M10
- **No kernel/ABI edits required.**
- **No renderer policy ownership violations.**
- **No framebuffer bounds weakening.**
- **sexdisplay remains sole framebuffer writer** [OK]

## 3. Contract Invariants

### I1: Framebuffer Writer Singularity
sexdisplay is the sole framebuffer writer. No other server writes to the
framebuffer. silkbar does not touch the framebuffer. silk-shell sends opcodes only.
**Verified:** [OK] No violations found.

### I2: SilkBar Model Authority
`silkbar-model` crate is the single source of truth for SilkBarUpdate ABI layout,
UpdateKind discriminants, SilkBar data structure, apply_update() semantics,
and contract validation gates.
**Verified:** [OK] Both producer and consumer import from model crate and call
validate_silkbar_contract() at _start().

### I3: Update Flow Direction
Updates flow producer -> consumer only: silkbar -> sexdisplay via PDX
OP_SILKBAR_UPDATE. No reverse channel. sexdisplay never mutates silkbar's state.
**Verified:** [OK] No reverse PDX calls from sexdisplay to SLOT_SILKBAR.

### I4: Top Strip Render Exclusivity
Rows y=0..49 are rendered exclusively by bar_color()/clock_fg_at()/bell_badge_at()
in sexdisplay. No surface can cover this region.
**Verified:** [OK] clamp_surface() enforces y >= BAR_H (50). redraw_top_strip()
never touches y>=51.

### I5: Shell Owns Policy, Display Owns Rendering
Silk-shell owns: focus policy, workspace switching, frame lifecycle, scene management,
chrome mode, appearance tokens.
Sexdisplay owns: pixel rendering, surface compositing, framebuffer management.
Neither crosses the boundary.
**Verified:** [OK] Shell sends opcodes; display renders. Display does not initiate
policy changes.

### I6: Update Queue Non-Overwrite
SilkBarUpdateQueue is a fixed ring buffer with UPDATE_QUEUE_CAP=32.
Full queue rejects new entries (push returns false). No overwrite.
**Verified:** [OK] push() returns false when (tail+1)%cap == head.

### I7: Bounds Safety
All update indices are checked against WORKSPACE_COUNT or MAX_CHIPS in
apply_update(). Out-of-bounds indices return false — no panic, no UB.
**Verified:** [OK] Every match arm that indexes into bar.workspaces[] or bar.chips[]
has a bounds check.

### I8: Clock Liveness Fallback
If silkbar stops sending SetClock for >5 seconds, sexdisplay resumes its local
clock. When silkbar recovers, it must send a non-stale time to regain ownership.
**Verified:** [OK] Fallback gate at sexdisplay line 1042-1053.

### I9: Appearance Token Application
Tokens applied as a two-call atomic state machine. Call 1 buffers, Call 2
commits all 8 colors + flags. Blur forced to 0.
**Verified:** [OK] Two-call state machine with TOKEN_BUF_CALL1_RECEIVED guard.
Alpha forced to 0xFF via clamp_color_token().

### I10: Surface Ownership Isolation
Only the owning PD (or registered WM) may mutate or destroy a surface.
Focus changes authorized for owner or WM. Non-owner ops rejected with
rate-limited logging.
**Verified:** [OK] All mutating ops (0xEB, 0xEC, 0xED, 0xEE, 0xEF, 0xFA, 0xFB, 0xFD)
check slot.owner_pd != msg.caller_pd.

## 4. Proof Markers / Gates

### Runtime Gates (verified present in source)

| Gate | Location | Marker |
|------|----------|--------|
| Model contract validation | silkbar _start() | `[silk.contract.validate.ok] version=2` |
| Model contract validation | sexdisplay _start() | `[silk.contract.validate.ok] version=2` |
| Render proof | sexdisplay after first OP_PRIMARY_FB | `[silk.render_proof.top_strip.ok]` |
| Render proof hash | sexdisplay top_strip_render_proof() | `[silk.render_proof.top_strip.hash] value=0x...` |
| Clock send proof | silkbar main loop | `[silkbar.clock.send] hh=... mm=... ss=...` |
| Clock fallback resume | sexdisplay | `[sexdisplay.clock.fallback.resume] reason=silkbar_stale` |
| Selected options forward | silkbar | `[silkbar.selected.options.forward] mask=...` |
| Selected options update | sexdisplay | `[sexdisplay.selected.options.update] mask=...` |
| Bell presence render | sexdisplay | `[sexdisplay.bell.render] total=... redacted=... flags=...` |
| Cursor Z-top proof | sexdisplay | `[sexdisplay.cursor_surface.z_top.ok]` |

### Static Compile-Time Gates

| Gate | Location | What it checks |
|------|----------|----------------|
| SILKBAR_UPDATE_SIZE == 16 | silkbar-model lib.rs:511 | ABI struct size |
| UPDATE_QUEUE_CAP == 32 | silkbar-model lib.rs:514 | Ring buffer capacity |
| ABI_VERSION > 0 | silkbar-model lib.rs:517 | Non-zero version |
| validate_contract() 15 checks | silkbar-model lib.rs:598-647 | All layout/dimension/index invariants |
| validate_deterministic_vectors() | silkbar-model lib.rs:651-700 | Update semantics: 7-vector test suite |

### Build-Time Verification

```sh
# Gate: static source analysis
./scripts/gate_render.sh

# Gate: full build with contract validation
./scripts/entrypoint_build.sh

# Gate: runtime proof (QEMU, requires boot)
./scripts/master_runtime_gate.sh --skip-build
```

## 5. Smallest Patch Prompt

### IMPLEMENT NOW: SILK_DE_CONTRACT_DRIFT_HARDEN_V1

Documentation + import consolidation. Zero behavioral change.

```
GOAL: Eliminate duplicated constant definitions across Silk DE servers
without changing any runtime behavior.

CONSTRAINTS:
- No renderer policy ownership changes.
- No framebuffer bounds changes.
- No broad refactor.
- No storage/Linen edits.
- No code implementation beyond import consolidation.

TASKS:

1. silk-shell: Import OPTION_* from silkbar-model
   File: servers/silk-shell/src/main.rs
   - Add OPTION_CLOSE, OPTION_ZOOM, OPTION_MINIMIZE, OPTION_MOVE to the
     existing `use silkbar_model::{...}` import block at line 14.
   - Remove local definitions at lines 4203-4209.
   - Verify: `rg "const OPTION_" servers/silk-shell/src/main.rs` empty.
   - Build: `cargo build -p silk-shell` must succeed.

2. silk-shell: Derive WORKSPACE_COUNT from silkbar-model
   File: servers/silk-shell/src/main.rs
   - Add SILKBAR_WORKSPACE_COUNT to the silkbar_model import.
   - Replace local `const WORKSPACE_COUNT: u8 = 5;` at line 4975 with:
     `const WORKSPACE_COUNT: u8 = SILKBAR_WORKSPACE_COUNT as u8;`
   - Build: `cargo build -p silk-shell` must succeed.

3. silkbar-model: Add document markers for ABI version dual tracking
   File: crates/silkbar-model/src/lib.rs
   - Add doc comment on SILKBAR_ABI_VERSION (line 71): "PDX wire protocol
     version (distinct from ABI_VERSION which governs model layout)."
   - Add doc comment on ABI_VERSION (line 56): "Model layout version.
     Consumers query SILKBAR_ABI_VERSION for PDX compat, not this."
   - No value changes.

4. silkbar-model: Add document marker for SetThemeToken no-op
   File: crates/silkbar-model/src/lib.rs
   - Above kind=5 match arm (line 427), add:
     // GATE: SetThemeToken (kind=5) is intentionally no-op in V1.
     // Theme tokens are delivered via OP_APPEARANCE_TOKENS (0xFC) to sexdisplay.
     // This slot reserved for future in-model theme storage.

5. sexdisplay: Add comment bridge for bar geometry
   File: servers/sexdisplay/src/main.rs
   - At BAR_H constant (line 165), add:
     // BAR_H covers model PANEL_Y+PANEL_H (10+38=48) + 2px safety margin.
     // Must remain >= PANEL_Y + PANEL_H from silkbar-model.
   - At y<50 render boundary (lines 717, 761), add:
     // Top strip boundary: BAR_H = 50. Keep in sync with BAR_H constant.

STOP CONDITIONS:
- Do NOT change any constant value.
- Do NOT change any render logic.
- Do NOT add new validation checks.
- Do NOT change OPTION_* or WORKSPACE_COUNT values.

VERIFICATION:
- Build: ./scripts/entrypoint_build.sh
- Gate: ./scripts/gate_render.sh
- Runtime: ./scripts/master_runtime_gate.sh --skip-build
- All existing runtime markers must still appear.
```

### DEFER: PLAN LATER (requires design, not implementation now)

```
FUTURE: SILK_DE_CONTRACT_SHARED_CHROME_V1
Goal: Move frame chrome constants to a shared crate to eliminate M5 duplication.
Approach: Extract SILK_CHROME_TEMPLATE_DEFAULT constants into silkbar-model as
pub const; sexdisplay and silk-shell import from model.
Risk: i32 vs usize type conflict needs resolution — silk-shell uses i32 for
hit-test, sexdisplay uses usize for array indexing.

FUTURE: SILK_DE_TOKEN_INDEX_ENUM_V1
Goal: Replace positional [u32; 8] TokenPreset with named enum (M7).
Approach: Define TokenSlot enum in silkbar-model with discriminants 0-7.
Wrap TokenPreset in a struct with named fields or accessors.

FUTURE: SILK_DE_MULTI_FOCUS_WIRE_V1
Goal: Wire focus_state 2 (app) and 3 (debug) through silk-shell (M6).
Approach: silk-shell tracks whether focus is on shell chrome, app frame,
or debug surface. Sends differentiated focus_state values to silkbar.
```

## 6. Implementation Prompt (Exact)

```
AUDIT: SILK_DE_CONTRACT_AUDIT_V1 COMPLETE.

FINDINGS: 3 MODERATE, 4 LOW, 3 INFO. No CRITICAL or HIGH.
No kernel/ABI edits required.
No renderer policy ownership violations.
No framebuffer bounds weakening.

STATUS: Proceed with SilkBar/sexdisplay model contract work.
Apply SILK_DE_CONTRACT_DRIFT_HARDEN_V1 (5 documentation/import patches)
before any behavior changes.

NEXT: Execute smallest patch prompt above (import consolidation + docs).
Then proceed with planned SilkBar model contract work on the hardened baseline.
```
