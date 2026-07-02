# STABLE BASELINE 2026-05-03

Canonical reference for what is proven and locked in the SexOS Silk DE stack.
Future agents should read this first before making any changes.

---

## 1. Proven Baseline

The following are **proven and stable** — regressions in these areas are blockers:

| Feature | Status | Last Verified |
|---------|--------|---------------|
| SexOS boots to shell | PASS | 2026-05-03 |
| PDs spawn with MPK isolation | PASS | 2026-05-03 |
| sexdisplay owns framebuffer | PASS | 2026-05-03 |
| SilkBar clock counts | PASS | 2026-05-03 |
| SilkBar ABI contract validates | PASS | 2026-05-03 |
| Top-strip render proof (FNV-1a hash) | PASS | 2026-05-03 |
| Renderer conformance cleanup | PASS | 2026-05-03 |
| Surface ID lifetime guards (shell-only) | PASS | 2026-05-03 |
| Click-focus (hit-test + dispatch) | PASS | 2026-05-03 |
| Drag-window (click-hold-move-release) | PASS | 2026-05-03 |
| SilkBar clickable controls (hit-test all 10 slots) | PASS | 2026-05-03 |
| Launcher panel toggle (id=0x92) | PASS | 2026-05-03 |
| Status panel toggle (id=0x93) | PASS | 2026-05-03 |
| Clock panel toggle (id=0x94) | PASS | 2026-05-03 |
| Workspace switching (click → silkbar → sexdisplay → redraw) | PASS | 2026-05-03 |
| USB xHCI/report route (interrupt-IN path) | PASS | 2026-05-03 |
| Synthetic downstream click proof (sexinput → shell → sexdisplay) | PASS | 2026-05-03 |
| Panel toggle consolidation (single `toggle_os_panel` helper) | PASS | 2026-05-03 |
| Bell panel toggle (id=0x95) | PASS | 2026-05-03 |

---

## 2. Locked Invariants

These invariants **must not be violated** without explicit STOP FIRST approval:

### Architecture
- **No kernel edits** without STOP FIRST. Kernel is the foundation — changes risk all PDs.
- **No sex-pdx ABI edits** without STOP FIRST. All inter-domain communication depends on this.
- **No PDX ABI changes** without STOP FIRST. The capability slot model is the sole IPC mechanism.
- **sexdisplay is sole framebuffer writer**. No other PD may write to the framebuffer. Shell owns policy; display owns pixels.

### SilkBar Contract
- **SilkBar producer/renderer contract must validate** at both ends (silkbar and sexdisplay).
- `validate_silkbar_contract()` must return 0 at startup. Reason codes: 1 = layout contract fail, 2 = deterministic vector fail.
- `SilkBarUpdate` is exactly 16 bytes (`#[repr(C)]`), asserted at compile time.
- `UPDATE_QUEUE_CAP` is exactly 32.

### Render
- **Top-strip render proof must pass** — `[silk.render_proof.top_strip.ok]` on every boot.
- The proof is an FNV-1a hash of rows 0..49 (50 rows of top strip).
- Only `redraw_top_strip()` (formerly `redraw_clock_only()`) touches y<50 pixels after initial render.
- Below-bar rendering (y≥50) uses `composite_pixel()` with two passes: non-focused surfaces, then focused surface on top.

### Shell
- **Shell owns policy** — surface positions, focus, z-order, and click dispatch.
- **No framebuffer writes from shell** — shell talks to sexdisplay via 0xEC/0xED/0xEE/0xEB opcodes only.
- No heap/std/thread/time APIs — strict `#![no_std]` with `extern crate alloc` for Vec.

### OS Surface ID Registry
Stable allocation. Do not reassign without updating this table and all handoffs.

| ID  | Surface        | Owner      | Purpose                         |
|-----|----------------|------------|---------------------------------|
| 0x90| Cursor         | OS (shell) | OS-owned cursor surface         |
| 0x92| Launcher panel | OS (shell) | App launcher panel (toggle)     |
| 0x93| Status panel   | OS (shell) | Quick settings / system state   |
| 0x94| Clock panel    | OS (shell) | Clock/calendar panel (toggle)   |
| 0x95| Bell panel     | OS (shell) | **Reserved** — not yet built    |
| 100 | SURFACE_ID_APP | App        | Example app surface             |
| 101 | SURFACE_ID_STATIC | App     | Example app surface             |
| 102 | SURFACE_ID_TEST3 | App     | Test surface                    |
| 103 | SURFACE_ID_TEST4 | App     | Test surface                    |
| 200 | SURFACE_ID_LINEN | App     | Linen test surface              |

---

## 3. Known Limitations

### USB/HID Hardware Proof
- **Physical USB tablet/mouse button proof is still environment-dependent.**
- Synthetic proof covers the downstream chain (sexinput → shell → sexdisplay) but uses programmed HID_EVENT messages, not physical USB interrupt-IN packets.
- QEMU's QMP/HMP/VNC/XTest synthetic host input does **not** reliably route to QEMU USB HID device models in the current configuration.
- Real hardware testing or USB passthrough (`-device usb-host`) is required for physical button proof.

### Synthetic Click Proof Scope
- The synthetic SilkBar click sequence in `sexinput` (`silkbar_click_stage`) proves the dispatch chain for launcher, workspace, status, and clock clicks.
- The synthetic USB mouse click sequence in `sexinput` (`synth_click_stage`) proves the click-focus path via USB mouse reports.
- Neither proves physical USB HID button-down edge detection.

### Panel Visuals
- All four toggled panels (launcher/status/clock/bell) are **solid-color rects** drawn by sexdisplay's generic surface path. No content/controls yet.
- Status panel is reserved for future quick settings. Clock panel for future calendar/time UI.
- Bell panel (0x95) toggles open/closed via Bell chip click — no notification content yet.

---

## 4. Standard Verification Command

```bash
# Build
./scripts/entrypoint_build.sh

# Run (capture serial)
SEXUSB_XHCI_TRACE=0 ./dev.sh run-nographic 2>/tmp/stable.trace | tee /tmp/stable.log

# Verify baseline markers
grep -aE "silk.contract|silk.render_proof|click_focus|shell.drag|shell.launcher|shell.status|shell.clock|silkbar.workspace|fault|panic|GP|PF" /tmp/stable.log | head -400

# Expected pass markers:
# [silk.contract.validate.ok] version=1
# [silk.render_proof.top_strip.ok]
# [shell.silkbar.click] target=launcher/status/clock/workspace
# [shell.launcher.open/close.ok]
# [shell.status.open/close.ok]
# [shell.clock.open/close.ok]
# [silkbar.workspace.active.send.ok]
# [shell.drag.start/move/end]
# NO [fault], [panic], [GP], [PF], [PAGE FAULT], [GENERAL PROTECTION]
```



## 5. NEXT_BOUNDARY_HARDENING_PLAN_V1

**Current solved risk:** SilkBar/sexdisplay ABI drift is mitigated by shared SilkBar contract validation, startup validation in silkbar + sexdisplay, top-strip render proof, and renderer conformance cleanup.

**New top risk:** feature coupling after contract lock.

**Hard Rule:** Every feature proves exactly one boundary.

**Anti-scope-creep Rule:** Reject patches touching USB + shell + display + kernel + sex-pdx together. Any patch spanning more than two major domains must STOP FIRST.

### Ordered Phases

#### A. USB_BUTTON_CLICK_PROOF_V1
- **Goal:** Prove physical USB button events reach the shell.
- **Allowed:** `sexusb`, `sexinput`
- **Forbidden:** `silk-shell`, `sexdisplay`, `kernel`
- **Invariants:** USB produces normalized input events only. No compositor/display policy. No framebuffer access.
- **Pass:** Physical button-down/up proof works (environment dependent).
- **STOP FIRST:** If kernel ABI changes are needed.

#### B. SHELL_FOCUS_CONTRACT_V1
- **Goal:** Formalize how shell routes input based on focus.
- **Allowed:** `silk-shell`, `sexinput`
- **Forbidden:** `sexdisplay`, `kernel`, apps
- **Invariants:** `silk-shell` owns pointer state, focus, placement, panel policy. No framebuffer writes. No app internals.
- **Pass:** Input events route to the correct focused surface ID without shell doing rendering.
- **STOP FIRST:** If display rendering logic is added to shell.

#### C. SURFACE_OWNERSHIP_CONTRACT_V1
- **Goal:** Formalize app/shell/display surface relationships.
- **Allowed:** `silk-shell`, `sexdisplay`
- **Forbidden:** `kernel`, apps (except minimal test stubs)
- **Invariants:** App/server requests surface. Shell places/focuses surface. Display renders surface. No raw framebuffer pointers exposed to apps. No shared backing-buffer redesign.
- **Pass:** Surface can be requested and rendered without sharing FB memory.
- **STOP FIRST:** If shared backing buffers are redesigned.

#### D. DOCK_OVERLAYBAR_MODEL_V1
- **Goal:** Add Dock/OverlayBar using existing paradigms.
- **Allowed:** `silkbar`, `silk-shell`, `sexdisplay`
- **Forbidden:** `kernel`, `sexusb`
- **Invariants:** Model as SilkBar/chrome modes. Do not create separate incompatible renderer protocols.
- **Pass:** Dock/OverlayBar renders using the established SilkBar/chrome contract.
- **STOP FIRST:** If a new IPC render protocol is proposed.

#### E. BELL_CAPABILITY_ATTENTION_V1
- **Goal:** Add capability-scoped notifications.
- **Allowed:** `silk-shell`, new bell service
- **Forbidden:** `sexdisplay` (direct), `kernel`
- **Invariants:** Capability-scoped notification/attention service first. No ordinary popup/toast system first. No app-spammable notifications.
- **Pass:** Notification triggers safely via capability.
- **STOP FIRST:** If apps can spam notifications without a cap.

#### F. LINEN_STATIC_SURFACE_V1
- **Goal:** Integrate Linen safely.
- **Allowed:** `linen`, `silk-shell`
- **Forbidden:** `kernel`, `sexdisplay` (internals)
- **Invariants:** Static surface first. No file mutation first. No app lifecycle redesign first.
- **Pass:** Linen renders a static surface via shell.
- **STOP FIRST:** If file mutation or app lifecycle changes are added.

### Boundary Rules Summary
- **USB:** Normalized input events only. No policy/FB access.
- **sexinput:** Normalize input events. Deliver over PDX. No shell/display policy.
- **silk-shell:** Owns pointer/focus/placement/panel policy. No FB writes, app internals, kernel ABI changes.
- **sexdisplay:** Renders pixels only. No input policy/app lifecycle. Preserve FB bounds checks.
- **Surface Ownership:** App requests, Shell places, Display renders. No raw FB pointers. No backing buffer redesign.
- **Dock/OverlayBar:** Model as SilkBar/chrome modes. No new renderer protocols.
- **Bell:** Capability-scoped attention service. No spam/ordinary popups.
- **Linen:** Static surface first. No file mutation or lifecycle redesign.

### Recurring Bug Handoff Rule
Every recurring bug/fix gets:
1. Symptom
2. Root cause
3. Invariant violated
4. Proof command
5. Fix pattern

### Validation Commands

```bash
# Verify all plan phases are documented
rg "NEXT_BOUNDARY_HARDENING_PLAN_V1|USB_BUTTON_CLICK_PROOF_V1|SHELL_FOCUS_CONTRACT_V1|SURFACE_OWNERSHIP_CONTRACT_V1|DOCK_OVERLAYBAR_MODEL_V1|BELL_CAPABILITY_ATTENTION_V1|LINEN_STATIC_SURFACE_V1" -n docs/ CLAUDE.md

# Verify docs-only changes (no code)
git diff --stat
git diff -- '*.md' '*.txt'

# Verify boundary compliance: no patch touches more than 2 major domains
# Major domains: kernel, sex-pdx, sexdisplay, silk-shell, sexinput, sexusb, apps
git diff --stat | grep -cE "kernel/|sex-pdx/|sexdisplay|silk-shell|sexinput|sexusb|apps/" | xargs test 3 -gt || echo "WARNING: patch touches 3+ domains"
```

### Handoff Index

| Document | Location |
|----------|----------|
| Allocator boot hang triage | `docs/handoff/ALLOCATOR_BOOT_HANG_TRIAGE_V1.md` |
| Bell panel toggle (BELL_PANEL_TOGGLE_V1) | `docs/handoff/BELL_PANEL_TOGGLE_V1.md` |
| Drag window proof | `docs/handoff/DRAG_WINDOW_PROOF_V1.md` |
| Panel toggle consolidation | `docs/handoff/PANEL_TOGGLE_CONSOLIDATION_V1.md` |
| SilkBar action slot expansion (Bell ABI) | `docs/handoff/SILKBAR_ACTION_SLOT_EXPANSION_V1.md` |
| Shell Global Interaction Contract | `docs/handoff/SHELL_GLOBAL_INTERACTION_CONTRACT_V1.md` |
| SilkBar clickable controls | `docs/handoff/SILKBAR_CLICKABLE_CONTROLS_V1.md` |
| Clock panel toggle | `docs/handoff/SILKBAR_CLOCK_PANEL_V1.md` |
| Status panel toggle | `docs/handoff/SILKBAR_STATUS_PANEL_V1.md` |
| Workspace switch through silkbar | `docs/handoff/SILKBAR_WORKSPACE_SWITCH_V1.md` |
| Stable baseline (this file) | `docs/handoff/STABLE_BASELINE_20260503.md` |

---

## 6. SHELL_GLOBAL_INTERACTION_CONTRACT_V1

**Failure hypothesis:** Individual phase proofs pass, but global UI behavior fails from event-order bugs, focus conflicts, chrome conflicts, surface ID ambiguity, or dead-PD dangling state.

### Required Subcontracts

#### A. SHELL_INTERACTION_STATE_V1
- **Goal:** Avoid endless scattered `*_ACTIVE` booleans. Define unified interaction state.
- **Owner:** `silk-shell`
- **Allowed:** `silk-shell`
- **Forbidden:** `sexdisplay`, `kernel`, `sexinput`, `silkbar`
- **Invariants:** Define idle / hover chrome / hover surface / pressing chrome / pressing surface / dragging surface / overlay active / dock active / bell attention pending / surface focused / no focused surface.
- **Pass:** State transitions correctly without overlapping conflicting states.
- **Negative Proof:** No two exclusive states can be active simultaneously. (Implementation later may be small enum/state table, not broad refactor).
- **STOP FIRST:** If broad refactor replaces small enum/state table.

#### B. HIT_TEST_PRIORITY_V1
- **Goal:** Define strict z-order and input capture hierarchy.
- **Owner:** `silk-shell`
- **Allowed:** `silk-shell`
- **Forbidden:** `sexdisplay`, `kernel`
- **Invariants:** Priority is exactly:
  1. emergency/system modal
  2. active OverlayBar
  3. armed Bell action surface
  4. GlobalBar/SilkBar chrome
  5. DockBar/EdgeBar chrome
  6. WindowBar chrome
  7. app surfaces topmost-first
  8. desktop/background
- **Pass:** Input routes exactly to the highest priority intersecting element.
- **Negative Proof:** Clicks on active OverlayBar never bleed through to app surfaces.
- **STOP FIRST:** If priority changes require kernel/display changes.

#### C. EVENT_ORDERING_CONTRACT_V1
- **Goal:** Deterministic event processing pipeline.
- **Owner:** `silk-shell`
- **Allowed:** `silk-shell`
- **Forbidden:** `sexdisplay`, `kernel`
- **Invariants:** Processing order must be: 1. receive bounded input events, 2. normalize/update pointer state, 3. hit-test, 4. update interaction state, 5. apply shell command/focus decision, 6. emit display/model updates, 7. yield.
- **Pass:** No display updates occur before hit-test and interaction state are finalized.
- **Negative Proof:** Out-of-order event dispatch is impossible.
- **STOP FIRST:** If pipeline requires multi-threaded or async locks inside the loop.

#### D. SURFACE_ID_LIFETIME_V1
- **Goal:** Safe and monotonic surface ID management.
- **Owner:** `silk-shell`
- **Allowed:** `silk-shell`, `sexdisplay`
- **Forbidden:** `kernel`, `sexinput`
- **Invariants:** No random permanent magic IDs for real apps. Early phases: monotonic IDs, no reuse. Dead IDs tombstoned. Focus cannot point to tombstoned surface. Unknown IDs fail safely. App death cannot leave dangling focused surface.
- **Pass:** App closure properly tombstones ID and clears focus.
- **Negative Proof:** Focus pointing to an invalid/dead ID does not crash the shell or display.
- **STOP FIRST:** If ID lifetime requires complex garbage collection.

#### E. CHROME_MODE_ARBITRATION_V1
- **Goal:** Strict rules for chrome visibility and input stealing.
- **Owner:** `silk-shell`
- **Allowed:** `silk-shell`, `silkbar`
- **Forbidden:** `sexdisplay`, `kernel`
- **Invariants:** GlobalBar passive unless clicked. OverlayBar exclusive while active. DockBar hover/click active but no focus steal unless launching. Bell requests attention; shell decides activation. WindowBar follows focused surface. Only one exclusive chrome mode at a time.
- **Pass:** Activating OverlayBar dismisses or overrides other exclusive chromes.
- **Negative Proof:** DockBar click does not steal focus from active app unless an app is launched.
- **STOP FIRST:** If chrome arbitration requires new IPC protocols.

#### F. DEAD_PD_SURFACE_CLEANUP_V1
- **Goal:** Safe teardown of crashed or closed PD surfaces.
- **Owner:** `silk-shell`
- **Allowed:** `silk-shell`, `sexdisplay`
- **Forbidden:** `kernel` (policy changes)
- **Invariants:** Tombstone/remove owned surfaces. Clear focus if focused surface died. Cancel drag if dragged surface died. Close WindowBar for dead surface. Optional Bell system notification. `sexdisplay` must not render dangling surface.
- **Pass:** Dead PD results in surface removal and safe display redraw.
- **Negative Proof:** Shell does not loop infinitely if display fails to acknowledge cleanup.
- **STOP FIRST:** If kernel policy changes are requested.

#### G. INTEGRATED_SCENARIO_PROOF_V1
- **Goal:** Real-world combined feature verification.
- **Owner:** System Integration
- **Allowed:** `silk-shell`, `sexinput`, `sexdisplay`
- **Forbidden:** `kernel`
- **Invariants:** Every future feature must prove one integrated scenario:
  - USB click focuses Linen surface while GlobalBar visible
  - status chip opens panel, second click closes, app focus unchanged
  - Bell pending does not steal focus during drag
  - Dock hover does not break surface click
  - killed app clears focus and does not crash display
  - OverlayBar captures input before app surface
- **Pass:** Scenario completes exactly as specified without side effects.
- **Negative Proof:** Interacting with unrelated UI during scenario does not break the chain.
- **STOP FIRST:** If scenario requires manual timing or race condition wins.

### Global Completion Rule
Every feature must prove all of the following before it is considered complete:

- **boundary proof** — feature touches exactly one domain boundary; no scope creep
- **negative proof** — the inverse case is safe (e.g. click outside dismisses, missing ID does not crash)
- **integration proof** — combined scenario passes (INTEGRATED_SCENARIO_PROOF_V1)
- **handoff proof** — symptom, root cause, invariant violated, proof command, fix pattern recorded
- **build proof** — `./scripts/entrypoint_build.sh` passes cleanly
- **boot/runtime proof with exact log markers** — boot log shows all required pass markers and zero fault/panic/GP/PF markers
- **fault scan pass** — `grep -cE "fault|panic|GP|PF|PAGE FAULT|GENERAL PROTECTION"` returns 0 in the boot log
- **forbidden diff scan pass** — `git diff` passes all invariant gates: no kernel edits without STOP FIRST, no sex-pdx edits without STOP FIRST, no framebuffer writes outside sexdisplay, no shell pixel writes, no std/libc/thread/POSIX imports, ≤2 major domains, no backing-buffer redesign

The `scripts/audit_invariant_gates.sh` script automates the forbidden diff scan. Run it before every commit.

### Anti-Scope Rule
- If a patch touches **USB + shell + display + kernel + sex-pdx** together, reject it and split.
- If a patch spans **more than two major domains** (kernel, sex-pdx, sexdisplay, silk-shell, sexinput, sexusb, apps), STOP FIRST before implementation.

### Validation Commands

```bash
# Verify all subcontracts are documented
rg "SHELL_GLOBAL_INTERACTION_CONTRACT_V1|SHELL_INTERACTION_STATE_V1|HIT_TEST_PRIORITY_V1|EVENT_ORDERING_CONTRACT_V1|SURFACE_ID_LIFETIME_V1|CHROME_MODE_ARBITRATION_V1|DEAD_PD_SURFACE_CLEANUP_V1|INTEGRATED_SCENARIO_PROOF_V1" -n docs/ CLAUDE.md

# Verify docs-only changes
git diff --stat
git diff -- '*.md' '*.txt'

# Verify boundary compliance
git diff --stat | grep -cE "kernel/|sex-pdx/|sexdisplay|silk-shell|sexinput|sexusb|apps/" | xargs test 3 -gt || echo "WARNING: patch touches 3+ domains"
```

---

## 7. Next Recommended Feature Order

Priority-ordered for minimum risk per step — aligned with NEXT_BOUNDARY_HARDENING_PLAN_V1 phases:

1. ~~**Bell panel toggle** (BELL_PANEL_TOGGLE_V1) — action proof only, uses `toggle_os_panel()` at reserved id=0x95.~~ **DONE** (PASS 2026-05-03)
2. **USB_BUTTON_CLICK_PROOF_V1** — physical USB button events. Allowed: sexusb, sexinput. Forbidden: silk-shell, sexdisplay, kernel.
3. **SHELL_FOCUS_CONTRACT_V1** — formalize input routing based on focus. Allowed: silk-shell, sexinput.
4. **SURFACE_OWNERSHIP_CONTRACT_V1** — formalize app/shell/display surface relationships. Allowed: silk-shell, sexdisplay.
5. **DOCK_OVERLAYBAR_MODEL_V1** — add dock using existing paradigms. Allowed: silkbar, silk-shell, sexdisplay.
6. **BELL_CAPABILITY_ATTENTION_V1** — capability-scoped notification service. Allowed: silk-shell, new bell service.
7. **LINEN_STATIC_SURFACE_V1** — integrate linen as static surface. Allowed: linen, silk-shell.
8. **Real status panel contents** — quick settings controls in the status panel.
9. **Launcher contents/actions** — app grid or shortcuts in the launcher panel.
10. **Keyboard HID** — keyboard input routing (separate from pointer).

---

## 8. Key Files Reference

| File | Role |
|------|------|
| `servers/silk-shell/src/main.rs` | Desktop shell / policy / window manager |
| `servers/silkbar/src/main.rs` | SilkBar producer (workspace, focus, clock, chips) |
| `servers/sexdisplay/src/main.rs` | Display server / renderer / compositor |
| `servers/sexinput/src/main.rs` | Input server / synthetic proof sequences |
| `servers/sexusb/src/main.rs` | USB xHCI driver / HID report parsing |
| `crates/silkbar-model/src/lib.rs` | Shared SilkBar model, contract, update queue, ABI types |
| `crates/sex-pdx/src/lib.rs` | PDX capability authority layer |
| `scripts/entrypoint_build.sh` | Deterministic build entrypoint |
| `dev.sh` | QEMU launch script |
| `docs/handoff/` | Per-feature handoff proofs |

---

## 9. Current Handoff Index

| Document | Proves |
|----------|--------|
| `BELL_PANEL_TOGGLE_V1.md` | Bell panel toggle (id=0x95) |
| `DRAG_WINDOW_PROOF_V1.md` | Click-hold drag on shell-managed surfaces |
| `SHELL_GLOBAL_INTERACTION_CONTRACT_V1.md` | 7 subcontracts for integrated UI behavior |
| `SILKBAR_CLICKABLE_CONTROLS_V1.md` | Hit-test dispatch for all 10 SilkBar slots |
| `SILKBAR_CLOCK_PANEL_V1.md` | Clock panel toggle (id=0x94) |
| `SILKBAR_STATUS_PANEL_V1.md` | Status panel toggle (id=0x93) |
| `SILKBAR_WORKSPACE_SWITCH_V1.md` | Workspace switching through full chain |
| `PANEL_TOGGLE_CONSOLIDATION_V1.md` | Consolidated toggle_os_panel() helper |
| `STABLE_BASELINE_20260503.md` | **This document** — canonical overview |

---

*End of baseline. Any agent modifying proven areas must update this document and all affected handoffs.*
