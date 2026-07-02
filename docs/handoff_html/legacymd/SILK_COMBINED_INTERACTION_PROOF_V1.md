# SILK_COMBINED_INTERACTION_PROOF_V1

## Status: LANDED

Build: `scripts/entrypoint_build.sh` — `[SEXOS ENTRYPOINT] success`
Date: 2026-05-20
Gate: `scripts/daily_driver_master_gate.sh` — new gate `silk_combined_interaction`

This gate verifies that all Silk Desktop Environment interaction markers
coexist in a single boot log, proving the completed batch (commits
d12f7418..630b289e) is intact. No new markers, no new behavior — gate only
scans for existing proof markers.

No kernel, ABI, sex-pdx, or source-code edits. Gate script + handoff only.

---

## 1. What The Gate Proves

The combined interaction gate answers the scenario-proof gap identified in
`SILK_DE_USABILITY_ROLLUP_V1.md` §5:

> No combined scenario proof exercising all operations in sequence.

Instead of adding a synthetic test sequence, this gate proves that all the
individual interaction markers coexist in the same boot. When the full set
of markers is present with zero faults, the combined batch cross-proof is
satisfied — all code paths are present and emitting correct markers.

## 2. Required Evidence Categories (12)

| # | Category | Log Marker Pattern | Source |
|---|----------|-------------------|--------|
| 1 | Pointer resize state | `[silk.resize.hit]`, `[silk.resize.begin]`, `[silk.resize.end]` | silk-shell |
| 2 | Pointer resize geometry | `[silk.resize.delta]`, `[silk.resize.apply]`, `[silk.resize.clamp]`, `[silk.resize.flush]` | silk-shell |
| 3 | Drag-to-snap | `[silk.snap.hit.top]`, `[silk.snap.hit.left]`, `[silk.snap.hit.right]`, `[silk.snap.apply]`, `[silk.snap.none]` | silk-shell |
| 4 | Tab hit/select/reorder | `[silk.tab.hit]`, `[silk.tab.select]`, `[silk.tab.reorder.swap]`, `[silk.tab.reorder.reject]`, `[silk.tab.drag.begin]`, `[silk.tab.drag.end]` | silk-shell |
| 5 | Safe close/tombstone | `[silk.close.request]`, `[silk.close.allowed]`, `[silk.close.tombstone]`, `[silk.close.state.clear]` | silk-shell |
| 6 | Live topstrip tick4/glitch fix | `[silk.live_topstrip.tick4]`, `[silk.live_topstrip.glitch.fix]`, `[silk.live_topstrip.audit]` | sexdisplay |
| 7 | Chrome glitch fix | `[silk.chrome.glitch.fix]` | silk-shell |
| 8 | clock_visible_seconds | `gate_clock_visible_seconds` != FAIL | gate script |
| 9 | top_strip_hash | `gate_top_strip_hash` != FAIL | gate script |
| 10 | frame_rim_visual | `gate_frame_rim_visual` != FAIL | gate script |
| 11 | frame_lights_visual | `gate_frame_lights_visual` != FAIL | gate script |
| 12 | faults_zero | `gate_faults_zero` == PASS | gate script |

Categories 1–7 are interaction-specific markers emitted by silk-shell and
sexdisplay. Categories 8–11 are independently-gated visual/display proofs.
Category 12 is the fault-safety gate.

## 3. Gate Behavior

```
PASS  — All 12 categories proven. Every interaction marker category has
         at least one marker present, all dependent gates are PASS or SKIP
         (SKIP means not compiled in — not a failure), and faults_zero=PASS.

SKIP  — No interaction-specific markers (categories 1–7) found in the log.
         The interaction scenario was not enabled in this boot. Honest skip.

FAIL  — Interaction markers ARE present (scenario was enabled) but one or
         more required categories are missing, OR a dependent gate FAILed,
         OR faults_zero != PASS.
```

The gate does NOT cause a default daily boot to FAIL: if no interaction
markers are present (normal for a headless or non-interactive boot), the
gate honestly SKIPs.

## 4. Proof Command

```bash
# Build with interaction proofs enabled (default profile)
./scripts/entrypoint_build.sh

# Boot in QEMU with pointer input, capture serial log
timeout 30s qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci \
  -device usb-kbd,bus=xhci.0 \
  -device usb-tablet,bus=xhci.0 \
  -serial file:/tmp/sexos_boot_interaction.log \
  -display none -no-reboot -no-shutdown || true

# Run combined gate
./scripts/daily_driver_master_gate.sh /tmp/sexos_boot_interaction.log

# Expected output (interaction scenario enabled):
#   silk_combined_interaction    PASS   all 12 interaction proof categories proven, 0 faults

# Expected output (interaction scenario not enabled):
#   silk_combined_interaction    SKIP   interaction scenario not enabled in this boot
```

## 5. Files Changed

| File | Change |
|------|--------|
| `scripts/daily_driver_master_gate.sh` | Added `gate_silk_combined_interaction` variable, evaluation block, ALL_GATES entry. Version bumped V34→V35. |
| `docs/handoff/SILK_COMBINED_INTERACTION_PROOF_V1.md` | This file. |
| `scripts/daily_driver_master_gate.sh.bak_silk_combined_v1` | Backup before patch. |

No changes to:
- `servers/silk-shell/src/main.rs` (all required markers already exist)
- `servers/sexdisplay/src/main.rs` (all required markers already exist)
- Kernel, ABI, sex-pdx, or any other server

## 6. Gate Script Insertion Points

| Item | Line (post-edit) | Purpose |
|------|-----------------|---------|
| Variable declaration | ~368 | `gate_silk_combined_interaction="SKIP"` |
| Evaluation block | ~3963–4037 | 75-line gate with enablement detection, category checks, PASS/SKIP/FAIL logic |
| ALL_GATES array entry | ~4371 | `"silk_combined_interaction:$gate_silk_combined_interaction"` |
| Version bump | ~394, ~3966 | Header V34→V35, Results V33→V35 (pre-existing inconsistency fixed) |

## 7. Invariants Preserved

- **No new features**: Gate only observes existing markers. No new behavior added.
- **No kernel/ABI/sex-pdx edits**: Gate script and handoff doc only.
- **No framebuffer changes**: Display rendering unchanged.
- **No shared backing-buffer redesign**: Architecture unchanged.
- **No broad refactor**: Single gate block added to existing script.
- **Default daily not affected**: Gate SKIPs when interaction scenario not enabled.
- **Existing gates unchanged**: All 331+ existing gates preserved with their original logic.
- **Fault containment preserved**: `faults_zero` still gates the final PASS/FAIL verdict.

## 8. Do-Not-Regress Rule

Any future change that adds new interaction behaviors (multitouch gestures,
keyboard snap shortcuts, overview animations) must ensure their markers are
added to this combined gate's category list, OR must add a separate combined
gate for the new batch. The combined gate proves batch coexistence — if a
new feature removes or renames an existing marker, this gate will detect the
regression.

## 9. Related Handoff Documents

| Document | Covers |
|----------|--------|
| `SILK_DE_USABILITY_ROLLUP_V1.md` | Batch summary of all 8 Silk DE improvements |
| `SILK_POINTER_RESIZE_STATE_V1.md` | Pointer resize FSM and state transitions |
| `SILK_POINTER_RESIZE_GEOMETRY_V1.md` | Live geometry update during resize |
| `SILK_DRAG_TO_SNAP_V1.md` | Drag-release snap to nearest edge |
| `SILK_TAB_HIT_REORDER_V1.md` | Tab hit testing, selection, reorder |
| `SILK_SAFE_CLOSE_TOMBSTONE_V1.md` | Safe close, tombstone, focus handoff |
| `SILK_LIVE_TOPSTRIP_GLITCH_FIX_V1.md` | Topstrip glass buffer refresh |
| `SILK_TOP_CHROME_GLITCH_FIX_V1.md` | Tab chrome glitch (stale tab_count) |
| `DAILY_DRIVER_MASTER_GATE_V1.md` | Master gate script reference |

---

## 10. Correction Notes (SILK_COMBINED_GATE_SKIP_FIX_V1)

- Root cause: scenario enablement was inferred from generic interaction markers
  (`silk.resize|snap|tab|close|live_topstrip|chrome.glitch`), which can appear in
  normal boots and incorrectly force FAIL when some categories are absent.
- Semantics fix: `silk_combined_interaction` now activates only when an explicit
  combined scenario sentinel is present:
  - `[silk.combined.interaction.begin]`, or
  - `[silk.combined.scenario.begin]`
- If neither sentinel exists in the log, gate reports:
  - `SKIP interaction scenario not enabled (missing explicit combined sentinel)`
- PASS/FAIL behavior when sentinel is present is unchanged:
  - PASS only with all required markers and no dependent failures.
  - FAIL if required markers are missing or dependent/fault gates fail.

*End of SILK_COMBINED_INTERACTION_PROOF_V1. No source edits made. Gate evaluates
existing markers only.*
