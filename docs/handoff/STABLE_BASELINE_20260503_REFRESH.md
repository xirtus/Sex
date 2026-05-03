# STABLE BASELINE 2026-05-03 (Refresh)

**Date:** 2026-05-03
**Status:** ACTIVE

## Current Known-Good Status

All stability patches merged and verified. System boots to a functional desktop with:
- SilkBar clock (local fallback with liveness timeout)
- Four app surfaces (100-103) + linen (200)
- Cursor surface (0x90) rendered as white arrow bitmap
- SilkBar clickable controls (launcher, workspace, status chip, clock, bell)
- Click-focus on app surfaces (shell owns focus policy)
- Drag-window proof (synthetic, one-shot)
- SilkBar panel toggles (open/close with proof markers)
- Focus contract enforced: panels/cursor never become focus
- Shell focus guarded: dead surfaces reject, nonfocusable surfaces reject
- Clock freeze eliminated: fallback stale-time gate + liveness timeout
- USB mouse real delta observable via budgeted markers

## Proof Counts (2026-05-03 nographic, 15s boot)

| Marker | Count | Status |
|--------|-------|--------|
| `sexinput.drag_proof.done` | 1 | ✅ One-shot synthetic drag proof |
| `shell.drag.start` | 1 | ✅ One-shot drag start |
| `sexinput.mouse.real.delta` | 15 | ✅ Budgeted (16), real deltas flowing |
| `shell.cursor.move` | 16 | ✅ Budgeted (16), cursor position tracked |
| `silk.contract.validate.ok` | 2 | ✅ Both silkbar + sexdisplay contract valid |
| `silk.render_proof.top_strip.ok` | 1 | ✅ Top-strip hash rendered |
| `silkbar.send_update.drop` | 0 | ✅ No dropped PDX messages |
| `sexdisplay.clock.fallback.resume` | 0 | ✅ No fallback resume (clock healthy) |
| `panic\|PAGE FAULT\|GENERAL PROTECTION` | 0 | ✅ No exceptions |

## Exact Commits and Tags

### Recent commits (newest first)

```
00d54ae fix(input): add budgeted markers for real cursor movement
019fd3a fix(shell): formalize focus contract with try_set_focus guard
8b066dd fix(display): resume fallback clock when SilkBar clock stalls
30536b7 chore(silkbar): refine PDX update drop diagnostics
df14d86 fix(silkbar): reject stale clock updates and preserve fallback
45421c4 fix(input): stop synthetic drag proof replay storm
490a521 chore(shell): clarify nonfocusable surface rejects
1bd138c fix(shell): guard dead and unknown surface ids
```

### All proof tags

```
proof-bell-panel-20260503
proof-bell-slot-abi-v2-20260503
proof-clock-fallback-gate-20260503
proof-cursor-arrow-shape-20260502
proof-cursor-real-input-m1-20260503
proof-cursor-surface-move-20260502
proof-cursor-z-top-20260502
proof-drag-window-20260503
proof-input-click-focus-chain-20260503
proof-input-replay-storm-fix-20260503
proof-m2-abi-asserts-20260503
proof-pdx-ring-overflow-diagnostic-20260503
proof-shell-focus-contract-20260503
proof-silkbar-liveness-fallback-20260503
proof-surface-id-lifetime-20260503
proof-surface-lifetime-micro-20260503
proof-usb-mouse-nonzero-20260502
proof-xhci-intr-ring-advance-20260502
```

## Stability Baseline (Completed)

| Patch | File(s) | Summary |
|-------|---------|---------|
| INPUT_REPLAY_STORM_FIX_V1 | sexinput/src/main.rs | One-shot gate for synthetic drag proof; stops replay every 120 ticks |
| CLOCK_FREEZE_FALLBACK_GATE_V1 | sexdisplay/src/main.rs, silkbar/src/main.rs | Stale-time gate rejects boot SetClock; fallback stays active |
| SILKBAR_LIVENESS_FALLBACK_V1 | sexdisplay/src/main.rs | 5-second liveness timeout; fallback resumes if silkbar clock stalls |
| SHELL_FOCUS_CONTRACT_V1 | silk-shell/src/main.rs | `try_set_focus()` guard wires `is_focusable_surface()` + alive check into all focus writes |
| SURFACE_ID_LIFETIME_PATCH_V1 | silk-shell/src/main.rs | `clear_focus_if_dead()`, `surface_is_alive()` covers all IDs, unknown-ID reject |
| PDX_RING_OVERFLOW_DIAGNOSTIC_V1 | silkbar/src/main.rs | Budgeted drop marker with kind/idx/err fields |
| CURSOR_REAL_INPUT_M1_DIAGNOSTIC | sexinput/src/main.rs, silk-shell/src/main.rs | Budgeted real-delta and cursor-move markers |
| Visual cursor (SDL) | — | Cursor arrow renders at correct position; synthetic proof clicks visible in SDL window |

## Canonical Build/Run Commands

```bash
# Build
./scripts/entrypoint_build.sh

# Nographic proof (serial-only, ~15s)
SEXUSB_XHCI_TRACE=0 timeout 15 ./dev.sh run-nographic \
  2>/tmp/proof.trace | tee /tmp/proof.log

# SDL window (visual, ~30s)
SEXUSB_XHCI_TRACE=0 timeout 30 ./dev.sh run

# Verify proof counts
grep -ac "sexinput.drag_proof.done" /tmp/proof.log
grep -ac "shell.drag.start" /tmp/proof.log
grep -ac "sexinput.mouse.real.delta" /tmp/proof.log
grep -ac "shell.cursor.move" /tmp/proof.log
grep -ac "silk.contract.validate.ok" /tmp/proof.log
grep -ac "silk.render_proof.top_strip.ok" /tmp/proof.log
grep -ac "silkbar.send_update.drop" /tmp/proof.log
grep -ac "sexdisplay.clock.fallback.resume" /tmp/proof.log
grep -acE "panic|PAGE FAULT|GENERAL PROTECTION" /tmp/proof.log

# Check tags
git tag -l 'proof-*' | sort
```

## Locked Invariants

These must NOT be violated by future patches:

1. **Cursor 0x90 is nonfocusable** — `is_focusable_surface()` returns false. `try_set_focus()` rejects it. `point_in_surface()` rejects it.
2. **Panels 0x92-0x95 are nonfocusable** — same guards as cursor.
3. **Focus writes go through `try_set_focus()` only** — no direct `FOCUSED_SURFACE_ID` assignments outside the guard.
4. **`clock_from_silkbar` is NOT a one-way latch** — liveness timeout and stale gate protect the fallback.
5. **sexdisplay is sole framebuffer writer** — no other server directly writes to FB.
6. **Shell owns focus policy** — sexdisplay only renders focus color, never decides focus.
7. **SPSC ring buffer can lose messages** — all display-side code must tolerate lost PDX messages.
8. **Synthetic proofs are one-shot** — drag proof does not replay. Silkbar clicks are bounded by tick.

## Next Steps (Future Feature Order)

Recommended order from STABLE_BASELINE_20260503.md:

1. **REAL_CLICK_TARGET_PROOF_V1** — Physical USB mouse click reaches target surface. Requires SDL window + physical mouse or uinput.
2. **REAL_DRAG_WINDOW_PROOF_V1** — Physical USB mouse drag moves window.
3. **SILKBAR_SELECTED_WINDOW_OPTIONS_V1** — SilkBar workspace menu shows per-window options.
4. **WINDOW_FRAME_NEON_RIM_V1** — Window decoration with neon border.
5. **FRAME_TAB_MODEL_V1** — Window tab management.

## Verification Command (Copy-Paste Ready)

```bash
./scripts/entrypoint_build.sh && \
SEXUSB_XHCI_TRACE=0 timeout 15 ./dev.sh run-nographic \
  2>/tmp/stable-baseline-refresh.trace | tee /tmp/stable-baseline-refresh.log && \
echo "=== PROOF COUNTS ===" && \
echo -n "drag_proof.done: " && grep -ac "sexinput.drag_proof.done" /tmp/stable-baseline-refresh.log && \
echo -n "shell.drag.start: " && grep -ac "shell.drag.start" /tmp/stable-baseline-refresh.log && \
echo -n "real.delta: " && grep -ac "sexinput.mouse.real.delta" /tmp/stable-baseline-refresh.log && \
echo -n "cursor.move: " && grep -ac "shell.cursor.move" /tmp/stable-baseline-refresh.log && \
echo -n "contract.ok: " && grep -ac "silk.contract.validate.ok" /tmp/stable-baseline-refresh.log && \
echo -n "render_proof.ok: " && grep -ac "silk.render_proof.top_strip.ok" /tmp/stable-baseline-refresh.log && \
echo -n "silkbar.drop: " && grep -ac "silkbar.send_update.drop" /tmp/stable-baseline-refresh.log && \
echo -n "fallback.resume: " && grep -ac "sexdisplay.clock.fallback.resume" /tmp/stable-baseline-refresh.log && \
echo -n "faults: " && grep -acE "panic|PAGE FAULT|GENERAL PROTECTION" /tmp/stable-baseline-refresh.log
```
