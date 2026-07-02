# SHELL_GLOBAL_INTERACTION_CONTRACT_V1

**Status:** ACTIVE (2026-05-03)
**Authoritative source:** `docs/handoff/STABLE_BASELINE_20260503.md` §6
**Canonical reference:** This document is a standalone extract of the baseline. The baseline is the authoritative source.

---

## Failure Hypothesis

Individual phase proofs pass, but global UI behavior fails from event-order bugs, focus conflicts, chrome conflicts, surface ID ambiguity, or dead-PD dangling state.

---

## Required Subcontracts (A–G)

### A. SHELL_INTERACTION_STATE_V1
- **Goal:** Avoid endless scattered `*_ACTIVE` booleans. Define unified interaction state.
- **Owner:** `silk-shell`
- **Allowed:** `silk-shell`
- **Forbidden:** `sexdisplay`, `kernel`, `sexinput`, `silkbar`
- **Invariants:** Define idle / hover chrome / hover surface / pressing chrome / pressing surface / dragging surface / overlay active / dock active / bell attention pending / surface focused / no focused surface.
- **Pass:** State transitions correctly without overlapping conflicting states.
- **Negative Proof:** No two exclusive states can be active simultaneously. (Implementation later may be small enum/state table, not broad refactor).
- **STOP FIRST:** If broad refactor replaces small enum/state table.

### B. HIT_TEST_PRIORITY_V1
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

### C. EVENT_ORDERING_CONTRACT_V1
- **Goal:** Deterministic event processing pipeline.
- **Owner:** `silk-shell`
- **Allowed:** `silk-shell`
- **Forbidden:** `sexdisplay`, `kernel`
- **Invariants:** Processing order must be: 1. receive bounded input events, 2. normalize/update pointer state, 3. hit-test, 4. update interaction state, 5. apply shell command/focus decision, 6. emit display/model updates, 7. yield.
- **Pass:** No display updates occur before hit-test and interaction state are finalized.
- **Negative Proof:** Out-of-order event dispatch is impossible.
- **STOP FIRST:** If pipeline requires multi-threaded or async locks inside the loop.

### D. SURFACE_ID_LIFETIME_V1
- **Status:** PARTIALLY IMPLEMENTED (SURFACE_ID_LIFETIME_PATCH_V1, 2026-05-03)
- **Goal:** Safe and monotonic surface ID management.
- **Owner:** `silk-shell`
- **Allowed:** `silk-shell`, `sexdisplay`
- **Forbidden:** `kernel`, `sexinput`
- **Invariants:** No random permanent magic IDs for real apps. Early phases: monotonic IDs, no reuse. Dead IDs tombstoned. Focus cannot point to tombstoned surface. Unknown IDs fail safely. App death cannot leave dangling focused surface.
- **Implemented guards:** `clear_focus_if_dead()` clears focus if it points to a dead surface. `clear_drag_if_dead()` cancels drag if drag target is dead. `surface_is_alive()` covers all known IDs (cursor, panels, app surfaces). Unknown IDs produce `[shell.surface.unknown.reject]` markers.
- **Not yet implemented:** Monotonic ID allocation (requires ABI change), tombstone registry, dead PD cleanup (requires kernel events).
- **Pass:** App closure properly tombstones ID and clears focus.
- **Negative Proof:** Focus pointing to an invalid/dead ID does not crash the shell or display.
- **STOP FIRST:** If ID lifetime requires complex garbage collection.

### E. CHROME_MODE_ARBITRATION_V1
- **Goal:** Strict rules for chrome visibility and input stealing.
- **Owner:** `silk-shell`
- **Allowed:** `silk-shell`, `silkbar`
- **Forbidden:** `sexdisplay`, `kernel`
- **Invariants:** GlobalBar passive unless clicked. OverlayBar exclusive while active. DockBar hover/click active but no focus steal unless launching. Bell requests attention; shell decides activation. WindowBar follows focused surface. Only one exclusive chrome mode at a time.
- **Pass:** Activating OverlayBar dismisses or overrides other exclusive chromes.
- **Negative Proof:** DockBar click does not steal focus from active app unless an app is launched.
- **STOP FIRST:** If chrome arbitration requires new IPC protocols.

### F. DEAD_PD_SURFACE_CLEANUP_V1
- **Goal:** Safe teardown of crashed or closed PD surfaces.
- **Owner:** `silk-shell`
- **Allowed:** `silk-shell`, `sexdisplay`
- **Forbidden:** `kernel` (policy changes)
- **Invariants:** Tombstone/remove owned surfaces. Clear focus if focused surface died. Cancel drag if dragged surface died. Close WindowBar for dead surface. Optional Bell system notification. `sexdisplay` must not render dangling surface.
- **Pass:** Dead PD results in surface removal and safe display redraw.
- **Negative Proof:** Shell does not loop infinitely if display fails to acknowledge cleanup.
- **STOP FIRST:** If kernel policy changes are requested.

### G. INTEGRATED_SCENARIO_PROOF_V1
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

---

## Global Completion Rule

Every feature must prove all of the following before it is considered complete:

- **boundary proof** — feature touches exactly one domain boundary; no scope creep
- **negative proof** — the inverse case is safe (e.g. click outside dismisses, missing ID does not crash)
- **integration proof** — combined scenario passes (INTEGRATED_SCENARIO_PROOF_V1)
- **handoff proof** — symptom, root cause, invariant violated, proof command, fix pattern recorded
- **build proof** — `./scripts/entrypoint_build.sh` passes cleanly
- **boot/runtime proof with exact log markers** — boot log shows all required pass markers and zero fault/panic/GP/PF markers
- **fault scan pass** — `grep -cE "fault|panic|GP|PF|PAGE FAULT|GENERAL PROTECTION"` returns 0 in the boot log
- **forbidden diff scan pass** — `git diff` passes all invariant gates: no kernel edits without STOP FIRST, no sex-pdx edits without STOP FIRST, no framebuffer writes outside sexdisplay, no shell pixel writes, no std/libc/thread/POSIX imports, ≤2 major domains, no backing-buffer redesign

---

## Anti-Scope Rule

- If a patch touches **USB + shell + display + kernel + sex-pdx** together, reject it and split.
- If a patch spans **more than two major domains** (kernel, sex-pdx, sexdisplay, silk-shell, sexinput, sexusb, apps), STOP FIRST before implementation.

---

## Validation

```bash
# Verify all subcontracts are documented
rg "SHELL_GLOBAL_INTERACTION_CONTRACT_V1|SHELL_INTERACTION_STATE_V1|HIT_TEST_PRIORITY_V1|EVENT_ORDERING_CONTRACT_V1|SURFACE_ID_LIFETIME_V1|CHROME_MODE_ARBITRATION_V1|DEAD_PD_SURFACE_CLEANUP_V1|INTEGRATED_SCENARIO_PROOF_V1" -n docs/ CLAUDE.md

# Verify docs-only changes
git diff --stat
git diff -- '*.md' '*.txt'

# Verify boundary compliance
git diff --stat | grep -cE "kernel/|sex-pdx/|sexdisplay|silk-shell|sexinput|sexusb|apps/" | xargs test 3 -gt || echo "WARNING: patch touches 3+ domains"
```
