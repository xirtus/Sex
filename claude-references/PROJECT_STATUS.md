# Project Status & Completed Work

> Referenced from CLAUDE.md (offloaded reference).
> For latest status, check recent commits and docs/handoff/STABLE_BASELINE_*.md.

---

## Current Status (last updated 2026-05-03 — INTERACTIVE_MODE_PROOF_GATE_V1)

- **Scheduler stall is FIXED.** All PDX domains spawn and schedule correctly.
- **USB HID boot-class mouse pipeline is code-complete.**
- **QEMU usb-tablet HID support (04566ab) — PROVEN:** Tablet HID detection, report descriptor scan, absolute position reports captured in SDL X11 session.
- **Workspace switch through silkbar PROVEN (SILKBAR_WORKSPACE_SWITCH_V1):** Workspace clicks update real active workspace state.
- **SilkBar clickable controls PROVEN (SILKBAR_CLICKABLE_CONTROLS_V1):** Shell hit-test for panel regions, synthetic proof clicks all four target types.
- **Drag-window proof PROVEN (DRAG_WINDOW_PROOF_V1):** Synthetic drag via HID_EVENT path.
- **Click-focus chain PROVEN (SYNTHETIC_CLICK_FOCUS_PROOF_V1):** Sexinput synthetic one-shot routes via OP_USB_MOUSE_REPORT → silk-shell.
- **Not yet proven via physical USB tablet:** Button events blocked by SDL2/XTest filter + QEMU 11.0 routing.
- **Full USB continuation status (blockers, audits, workarounds, next steps):** See `claude-references/USB_STATUS.md`.

---

## Interactive Mode Proof Gate

- Three hardcoded `USB_PROOF_DISABLE_*` consts merged into single `SYNTHETIC_INPUT_PROOFS_DISABLED` gate
- Uses `option_env!("SEXOS_PROOFS_DISABLED")` — set env var at build time to disable proofs for interactive use
- Default (unset): proofs enabled for CI/nographic verification
- Zero proof code removed — all blocks conditionally gated
- No kernel/PDX/ABI changes
- See `docs/handoff/INTERACTIVE_MODE_PROOF_GATE_V1.md`

---

## Completed Features

### M2 audit assert patch (SILK_DE_M2_ASSERT_PATCH_V1)
- F3: sexdisplay apply_update() return value now captured; invalid updates logged and do NOT trigger redraw
- F4: ChipSlot discriminant invariant added to validate_contract()
- Files: `crates/silkbar-model/src/lib.rs` (+8 lines), `servers/sexdisplay/src/main.rs` (+6/-6 lines)

### Real click target proof (REAL_CLICK_TARGET_PROOF_V1)
- **Fix 1 — Double-apply eliminated:** EV_REL now owns cursor movement exclusively
- **Fix 2 — Coordinate corruption fixed:** Synthetic click-focus proof uses EV_ABS(940,560)
- **Fix 3 — EV_BTN owns click targeting:** Full click-target markers added to EV_BTN handler
- Files: `servers/sexinput/src/main.rs`, `servers/silk-shell/src/main.rs`

### Renderer conformance cleanup (RENDERER_CONFORMANCE_CLEANUP_V1)
- 11 magic color literals replaced with DEFAULT_THEME fields
- Top-strip hash confirmed unchanged: `0x3c8d391f6e312fca`

### Top-strip render proof (SILK_TOP_STRIP_RENDER_PROOF_V1)
- FNV-1a hash over rows 0..50 after first live render
- Hash printed atomically (single pdx_call)
- Baseline hash: `0x3c8d391f6e312fca` (QEMU virtio-gpu, 1280 wide, default bar state)

### SilkBar contract locked (SILK_DE_CONTRACT_LOCK_V1)
- `validate_silkbar_contract() -> u32` added to silkbar-model
- Both silkbar and sexdisplay emit `[silk.contract.validate.start/ok/fail]` markers at `_start`

### Input replay storm fix (INPUT_REPLAY_STORM_FIX_V1)
- Synthetic drag proof no longer wraps forever via `% 3`
- One-shot gate prevents replay after stage 2

### Clock freeze fallback gate (CLOCK_FREEZE_FALLBACK_GATE_V1)
- SilkBar clock no longer freezes at 00:00 within 4s
- Added stale-time gate in sexdisplay

### SilkBar liveness fallback (SILKBAR_LIVENESS_FALLBACK_V1)
- `clock_from_silkbar` no longer a permanent one-way latch
- 5-second timeout before fallback resumes

### Shell focus contract (SHELL_FOCUS_CONTRACT_V1)
- All focus write paths routed through `try_set_focus()` guard
- `is_focusable_surface()` wired into all focus write paths

### Cursor real input diagnostic (CURSOR_REAL_INPUT_M1_DIAGNOSTIC)
- Budgeted markers in sexinput's real USB mouse path and shell's cursor surface update

---

## Next Action Options

**USB input continuation:** See `claude-references/USB_STATUS.md` for the full
blocker analysis, audit results, workarounds, and three concrete next-step options
(physical mouse proof, uinput virtual mouse, re-enable synthetic clicks).

Other upcoming phases (from stable baseline):
1. USB_BUTTON_CLICK_PROOF_V1
2. SHELL_FOCUS_CONTRACT_V1
3. SURFACE_OWNERSHIP_CONTRACT_V1
4. DOCK_OVERLAYBAR_MODEL_V1
5. BELL_CAPABILITY_ATTENTION_V1
6. LINEN_STATIC_SURFACE_V1
