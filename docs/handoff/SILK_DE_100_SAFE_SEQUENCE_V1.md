# SILK_DE_100_SAFE_SEQUENCE_V1

Date: 2026-07-05
Scope: silk-shell, sexdisplay, silkbar, silkbar-model, gate scripts, this doc.
Out of scope (STOP FIRST required): kernel, sex-pdx, ABI wire changes, new PDX opcodes,
shared backing-buffer redesign, any framebuffer writer other than sexdisplay.

---

## A. Current Status (audited 2026-07-05)

### crates/silkbar-model (897 lines) — contract layer ALREADY STRONG
- `validate_silkbar_contract()` → 0/1/2 reason codes; checks magic `SDBA`,
  ABI v4, LAYOUT_COUNT 11, THEME 10 tokens, ChipSlot discriminant pinning,
  `SilkBarUpdate` == 16 bytes, queue cap 32, geometry bounds (PANEL_W ≤ 4096, PANEL_H ≤ 256).
- `contract_fingerprint()` → u64 xor-fold of all contract constants.
- `validate_deterministic_vectors()` → 10-vector headless update-semantics gate
  (workspace/chip/clock/options/phase1 SetActiveApp/SetTintAccent/SetPaletteState).
- Compile-time asserts: update size 16, queue cap 32, ABI_VERSION > 0.
- `UpdateKind::SetThemeToken (5)` intentionally no-op — theme travels via
  OP_APPEARANCE_TOKENS (0xFC), 8 named token indices (`APPEARANCE_TOKEN_*`).
- PDX opcodes reserved: 0xF0 ping, 0xF1 get-abi, 0xF2 update.

### servers/sexdisplay (3628 lines) — renderer conformance MOSTLY DONE
- Calls `validate_silkbar_contract()` at startup; emits pass/fail with fingerprint.
- `top_strip_render_proof()` (main.rs:1671): renders fixed 6-update vector into
  bounded 1280×51 offscreen buffer, FNV-1a hashes it.
  **Golden hash LOCKED: `SILK_DE_TOPSTRIP_EXPECTED_HASH = 0x9b5d54e17bdfa6f1`**
  (main.rs:1764). Strict pass/fail active — no longer observe mode.
  Runs once after OP_PRIMARY_FB, gated by `SILK_RENDER_PROOF_PROFILE_ENABLED`.
- `emit_renderer_conformance_marker()` (main.rs:1768): contract ok + fingerprint
  nonzero + expected hash set + proof dims bounded → `[silk.de.renderer.conformance.pass]`.
- Frame chrome rendering exists: `FRAME_RIM_PX = 8`, `FRAME_RIM_COLOR = 0x00B4BEFE`,
  chrome_flags transport already defined:
  bit 0 top-bar, bit 1 `SURFACE_CHROME_FRAME_HOVER`, bit 2 `SURFACE_CHROME_LIGHT_HOVER`,
  bits 3-4 hovered light kind. Renderer is state→pixels only. Bounds checks intact.

### servers/silk-shell (25317 lines) — model skeleton LARGELY PRESENT
- Structs exist: `SceneId(u8)`, `FrameId(u32)`, `TabIndex(u8)`, `Scene`,
  `SceneDescriptor`, `WindowState`, `FrameSnapshot`, `SceneLayoutSnapshot`,
  `AtlasSnapshot`, `AtlasFrameSnapshot`, `AtlasSceneSnapshot`, `AtlasDragIntent`,
  `SceneAppearanceState`.
- Hover state machine present: `HOVER_NONE / HOVER_FRAME_BODY / HOVER_FRAME_RIM /
  HOVER_TAB_STRIP` kinds; `HOVERED_FRAME_ID`, `HOVERED_FRAME_LIGHT`,
  `update_frame_hover_at()` per event-loop iteration. `HOVER_FRAME_RIM` marked
  "future: neon rim" — kind defined, not yet driving visuals.
- Atlas proofs env-gated: `SEXOS_ATLAS_OVERVIEW_PROOF`, `_SCENE_KEYBOARD_`,
  `_THEME_VISUAL_`, `_THEME_PRESETS_`, `_SCENE_STUB_`, `_PREVIEW_`.
- Integrated interaction markers: `[silk.de.integrated.interaction.pass]`.
- **GAP 1:** silk-shell (producer of SetActiveApp/SetTintAccent/SetPaletteState and
  appearance tokens) never calls `validate_silkbar_contract()` — no producer-side
  contract stamp. silkbar and sexdisplay both validate; the third party does not.
- **GAP 2 — PROOF THEATER:** `maybe_run_frame_rim_markers_proof()` (main.rs:2582)
  emits HARDCODED `[silk.frame.rim.state]` strings — constants, not derived from
  frame state. A gate reading these proves nothing. Must be replaced in Phase E.

### servers/silkbar (804 lines)
- Validates contract at `_start` with fingerprint marker; producer of clock/bell
  updates via OP_SILKBAR_UPDATE. Clean, small, in-contract.

### Gates
- `daily_driver_master_gate.sh` rows 89 `silk_de_topstrip_deterministic` and
  89b `silk_de_renderer_conformance` exist. Both default SKIP unless begin
  markers appear — i.e. the silk-de lane is only exercised when a run is built
  with the render-proof profile. **GAP 3:** no dedicated always-on silk-de lane;
  gate rows silently SKIP in normal runs.

---

## B. Safest Phase Sequence

Ordering principle: lock contracts and make existing gates non-skippable BEFORE
any new visuals; every phase proves exactly ONE boundary; model changes precede
renderer changes; renderer changes never add policy.

| Phase | Name | Status today | One boundary proved |
|-------|------|--------------|---------------------|
| A | Contract lock close-out | 90% done | model ↔ all three consumers agree on fingerprint |
| B | Renderer conformance lane | 80% done | sexdisplay renders model, nothing else |
| C | Top-strip golden hash | LOCKED | deterministic pixels from deterministic state |
| D | Frame/Tab model proof | structs exist, no proof | shell model ops are deterministic, render-free |
| E | Hover rim/tab visual V1 | transport exists, visuals stub | chrome_flags → rim brightness, renderer stays dumb |
| F | Atlas proof-only skeleton | proofs exist, scattered | Atlas model math, zero runtime rendering |

### Phase A — Contract Lock Close-out
- **Change:** silk-shell calls `validate_silkbar_contract()` +
  `contract_fingerprint()` once at startup; emit
  `[silk.de.contract.producer] ok=<0|1> fp=0x<16x>`.
- **Files:** `servers/silk-shell/src/main.rs` (one small block near `_start`),
  `scripts/daily_driver_master_gate.sh` (new row: producer fingerprint must
  equal renderer fingerprint when both markers present).
- **Negative test:** temporarily bump `SILK_DE_BAR_ABI_V1` mismatch in a scratch
  build → all three servers must emit contract fail markers, gate row FAIL.
- **STOP FIRST if:** the fix requires touching silkbar-model constants
  themselves (that is a contract change, not a close-out).

### Phase B — Renderer Conformance Lane (make gate non-skippable)
- **Change:** dedicated `scripts/silk_de_gate.sh` (or a lane inside
  `gate_0_2.sh`) that builds with the render-proof profile env, boots, and
  requires rows 89/89b to be PASS — SKIP counts as FAIL in this lane.
- **Files:** new `scripts/silk_de_gate.sh`; no server code.
- **Negative test:** run lane against a build without the proof profile →
  lane must report FAIL (missing begin marker), not PASS.
- **STOP FIRST if:** lane needs new serial markers from sexdisplay beyond
  what exists (would mean Phase B scope creep into renderer code).

### Phase C — Golden Hash Discipline (already locked; document + negative test)
- **Change:** none to code. Add re-baseline procedure below + a tamper check in
  `silk_de_gate.sh`: assert source still contains the locked constant
  `0x9b5d54e17bdfa6f1` (grep), so silent re-baselines get flagged in review.
- **Re-baseline procedure (only legitimate reason: intentional theme/layout change):**
  1. Set `SILK_DE_TOPSTRIP_EXPECTED_HASH = 0` (observe mode).
  2. Boot proof lane, capture `[silk.de.topstrip.proof.observe] hash=...`.
  3. Paste new hash, re-run lane, require `.pass`.
  4. Record old→new hash + reason in this doc's changelog.
- **Negative test:** mutate one proof vector (e.g. clock 10:27:43) in scratch
  build → `.fail` marker with expected/got pair.
- **STOP FIRST if:** hash instability appears across identical builds — that is
  nondeterminism in the renderer (uninitialized buffer, timing leak), a bug,
  never a re-baseline.

### Phase D — Frame/Tab Model Proof (model only, no render)
- **Change:** env-gated `SEXOS_SILK_FRAME_MODEL_PROOF=1` synthetic proof in
  silk-shell driving REAL model functions (not prints): create frame → focus →
  add tab → cycle tab → close tab → close frame, on `Scene`/`FrameId`/`TabIndex`.
  Emit per-step `[silk.frame.model.step] op=<op> frame=<id> tabs=<n> focused=<id> ok=1`
  and `[silk.frame.model.proof.pass] steps=<n>` derived from actual state reads.
- **Files:** `servers/silk-shell/src/main.rs` only; gate row in
  `daily_driver_master_gate.sh` + lane in `silk_de_gate.sh`.
- **Negative tests:** close nonexistent frame → `ok=0 reason=no_such_frame`
  step marker, proof still passes (rejection is correct behavior);
  tab index past capacity → rejected, state unchanged (assert via re-read).
- **STOP FIRST if:** proof requires new surface ops to sexdisplay or any
  buffer allocation change. Model proof must be pure state manipulation.

### Phase E — Hover Rim/Tab Visual V1
- **Change (shell, policy side):** wire `HOVER_FRAME_RIM` kind in
  `update_frame_hover_at()` (pointer within `FRAME_RIM_PX` band of focused-scene
  frame); set/clear `SURFACE_CHROME_FRAME_HOVER` bit in chrome_flags it already
  sends. **Delete the hardcoded `maybe_run_frame_rim_markers_proof()` strings**;
  re-emit same marker names derived from real `HOVERED_FRAME_ID`/hover kind.
- **Change (renderer, dumb side):** sexdisplay: when
  `SURFACE_CHROME_FRAME_HOVER` set, draw rim with `lighten(frame_rim_color)`
  (reuse silkbar-model `lighten()`); else current color. No hit testing, no
  state, no new bounds math beyond existing rim path.
- **Files:** `servers/silk-shell/src/main.rs`, `servers/sexdisplay/src/main.rs`
  (two domains — at the STOP FIRST limit, allowed, not exceedable).
- **Markers:** `[shell.hover.rim] frame=<id> enter=1|0`,
  `[sexdisplay.rim.hover.draw] sid=<id> lit=1|0` (budgeted, not per-frame spam —
  see perf log budget lesson in PERF_LOG_NOISE_ABLATION_V1).
- **Negative tests:** hover with no focused frame → no bit set, no lit draw;
  hover on minimized frame → bit must not set; kill hovered surface →
  `[shell.hover.clear.dead]` (already exists, keep as regression marker).
- **STOP FIRST if:** V1 needs per-pixel alpha/glow blending (new fill paths in
  renderer) — V1 is flat color swap only; or if tab-strip hover is tempting —
  that is `HOVER_TAB_STRIP`, a later phase.

### Phase F — Atlas Proof-Only Skeleton (consolidation)
- **Change:** no new features. One umbrella lane in `silk_de_gate.sh` running
  existing `SEXOS_ATLAS_OVERVIEW_PROOF` + `SEXOS_ATLAS_SCENE_STUB_PROOF`
  builds; single summary row `silk_de_atlas_skeleton` requiring their pass
  markers and requiring ZERO `[sexdisplay.*atlas*]` draw markers (Atlas must
  not render in V1).
- **Files:** `scripts/silk_de_gate.sh`, `scripts/daily_driver_master_gate.sh`.
- **Negative test:** grep boot log for any atlas draw marker → presence FAILS
  the row (proof-only invariant).
- **STOP FIRST if:** any Atlas proof turns out to write surfaces/framebuffer —
  that is a runtime feature, not a skeleton, and needs its own sequence doc.

---

## C. Ownership Boundaries (exact)

| Component | Owns | Must never |
|-----------|------|------------|
| silkbar-model | types, constants, `apply_update`, contract validation, fingerprint | render, transport, hold runtime state |
| silk-shell | focus/hover/drag/scene/tab/session POLICY; produces updates, appearance tokens, chrome_flags | write framebuffer; know pixel colors beyond token indices |
| sexdisplay | SOLE framebuffer writer; pure state→pixels; bounds checks | decide focus/hover/policy; mutate model except via `apply_update` drain |
| silkbar | clock/bell/chip update producer | render; own layout |
| kernel / sex-pdx | untouched this entire sequence | — |

Transport stays as-is: OP_SILKBAR_UPDATE 0xF2, OP_APPEARANCE_TOKENS 0xFC,
existing surface-update chrome_flags bits. Any new opcode = STOP FIRST.

## D. Proof Gate List

| Gate row | Markers required | Phase |
|----------|-----------------|-------|
| silk_de_contract_producer (new) | `[silk.de.contract.producer] ok=1 fp=…` == renderer fp | A |
| silk_de_renderer_conformance (89b) | `conformance.begin` + `.pass`, no `.fail` | B |
| silk_de_topstrip_deterministic (89) | `proof.begin` + `proof.pass`, no `.fail`; source grep for locked hash | C |
| silk_de_frame_model (new) | `frame.model.proof.pass`, per-step ok markers, negative-op rejects | D |
| silk_de_hover_rim (new) | `[shell.hover.rim]` + `[sexdisplay.rim.hover.draw] lit=1`; none when unfocused | E |
| silk_de_atlas_skeleton (new) | existing atlas proof passes; zero atlas draw markers | F |

All rows live in `daily_driver_master_gate.sh`; `silk_de_gate.sh` is the lane
that builds the proof profile and treats SKIP as FAIL for silk-de rows.

## E. Codex-Ready Implementation Prompts

### Prompt A
```
TASK: SILK_DE Phase A — producer contract stamp.
FILES: servers/silk-shell/src/main.rs, scripts/daily_driver_master_gate.sh.
In silk-shell startup (near _start init, after PDX ready), call
silkbar_model::validate_silkbar_contract() and contract_fingerprint().
Emit: [silk.de.contract.producer] ok=<0|1> err=<code> fp=0x<016X>
Add gate row silk_de_contract_producer: SKIP if marker absent; FAIL if ok=0
or if producer fp differs from the fp printed by sexdisplay's contract marker;
PASS otherwise. Do NOT modify crates/silkbar-model. Do NOT add opcodes.
No framebuffer access. Build check: cargo build for silk-shell only.
```

### Prompt B
```
TASK: SILK_DE Phase B — dedicated conformance lane.
FILES: new scripts/silk_de_gate.sh only. No Rust changes.
Copy the boot/QEMU harness pattern from scripts/gate_0_2.sh. Build with the
render-proof profile env (same env that sets SILK_RENDER_PROOF_PROFILE_ENABLED
in sexdisplay — find it with rg 'SILK_RENDER_PROOF' in the build scripts).
Boot, capture serial, then require:
  [silk.de.renderer.conformance.pass]
  [silk.de.topstrip.proof.pass]
Missing begin markers = FAIL (not SKIP). Also grep source:
  rg -q '0x9b5d54e17bdfa6f1' servers/sexdisplay/src/main.rs || FAIL
Exit nonzero on any FAIL. Print one summary table.
```

### Prompt D
```
TASK: SILK_DE Phase D — frame/tab model synthetic proof.
FILES: servers/silk-shell/src/main.rs, scripts/daily_driver_master_gate.sh,
scripts/silk_de_gate.sh.
Add env-gated proof (SEXOS_SILK_FRAME_MODEL_PROOF=1, option_env pattern like
ATLAS_OVERVIEW_PROOF_ENABLED). Drive REAL model state: create frame in active
Scene, focus it, add tab, cycle tab, close tab, close frame. After each op,
READ BACK state and emit:
  [silk.frame.model.step] op=<name> frame=<id> tabs=<n> focused=<id> ok=<0|1>
Include two negative ops: close nonexistent FrameId (expect ok=0
reason=no_such_frame, state unchanged), add tab past capacity (expect ok=0).
Finish: [silk.frame.model.proof.pass] steps=<n> rejects=2
HARD RULE: no serial_println of constants — every field read from state.
No surface ops, no sexdisplay messages, no rendering. Gate row: FAIL on any
ok=0 for positive ops, FAIL if rejects!=2, PASS on proof.pass marker.
```

### Prompt E
```
TASK: SILK_DE Phase E — hover rim visual V1.
FILES: servers/silk-shell/src/main.rs, servers/sexdisplay/src/main.rs. TWO
domains max — touch nothing else.
Shell: in update_frame_hover_at(), classify pointer in outer FRAME_RIM_PX band
of a focusable, non-minimized frame in active scene as HOVER_FRAME_RIM; set
SURFACE_CHROME_FRAME_HOVER bit in the chrome_flags already sent for that
surface; clear on exit/minimize/death. DELETE hardcoded strings in
maybe_run_frame_rim_markers_proof(); re-emit same marker names from real
HOVERED_FRAME_ID/HOVER_KIND state. Budget hover markers (no per-move spam).
Display: in existing rim draw path, if SURFACE_CHROME_FRAME_HOVER set use
silkbar_model::lighten(frame_rim_color), else current color. Flat color swap
only — no glow, no alpha, no new fill loops, keep every bounds check.
Markers: [shell.hover.rim] frame=<id> enter=<0|1>;
[sexdisplay.rim.hover.draw] sid=<id> lit=<0|1> (budgeted).
Negative: hover over minimized frame must not set bit.
STOP FIRST if this needs a new opcode or chrome_flags bit beyond bit 1.
```

### Prompt F
```
TASK: SILK_DE Phase F — atlas proof-only umbrella lane.
FILES: scripts/silk_de_gate.sh, scripts/daily_driver_master_gate.sh only.
Add lane section building with SEXOS_ATLAS_OVERVIEW_PROOF=1 and
SEXOS_ATLAS_SCENE_STUB_PROOF=1 (find their pass markers with
rg 'atlas.*proof' servers/silk-shell/src/main.rs). Row silk_de_atlas_skeleton:
PASS requires those pass markers AND zero matches for atlas draw markers from
sexdisplay (rg -i 'sexdisplay.*atlas' on serial log must be empty). Any atlas
rendering evidence = FAIL. No Rust changes permitted in this phase.
```

(Phase C needs no implementation prompt — code locked; procedure in §B.)

## F. Changelog
- 2026-07-05: Initial audit + sequence. Golden hash locked at
  `0x9b5d54e17bdfa6f1` (found already set). Identified hardcoded rim markers
  proof (silk-shell:2582) as proof theater — scheduled for replacement in
  Phase E. Identified missing producer-side contract validation in silk-shell
  (Phase A close-out).
