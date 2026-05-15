# OVERNIGHT_PLANS_AND_PROMPTS_V1

## Goal
Prepare ready-to-run prompts for 10 bounded SexOS missions without implementing the missions in this step.

## Baseline
- `./scripts/run_daily_driver_proof.sh` currently expected: `18/18 PASS`, `faults=0`
- Keyboard-first daily-driver V1 proven
- SilkBar ABI Phase 1-5 proven
- Pointer/USB/slot2 mouse remains deferred

## Mission Order, Risk, Dependencies
1. `APP_LAUNCHER_VISUAL_KEYS_HELP_V1` (Risk: Low)
- Depends on: current shell palette marker paths in `silk-shell`
- Notes: marker/small UI only, no layout redesign

2. `SPINDLE_COMMAND_ALIASES_V1` (Risk: Low)
- Depends on: stable Spindle command dispatch path
- Notes: parser-local aliases only

3. `LINEN_SEARCH_FILTER_KEYBOARD_V1` (Risk: Medium)
- Depends on: seeded local Linen object table
- Notes: no blocking open, no PDX wait

4. `BELL_EVENT_FILTER_KEYBOARD_V1` (Risk: Medium)
- Depends on: local Bell event ring + keyboard nav
- Notes: preserve detail flow

5. `ATLAS_THEME_PREVIEW_MARKERS_V1` (Risk: Medium)
- Depends on: Atlas scene/accent state model
- Notes: marker-first preview, no renderer redesign

6. `HANDOFF_INDEX_AND_PROOF_REGISTRY_V1` (Risk: Low, Docs-only)
- Depends on: existing handoff/proof docs
- Notes: docs consistency mission

7. `DAILY_DRIVER_MASTER_GATE_HARDENING_V1` (Risk: Medium)
- Depends on: current gate scripts + baseline pass profile
- Notes: script hardening only if needed, preserve current PASS behavior

8. `APP_INSTALL_MODEL_PLAN_V1` (Risk: Medium, Docs-only)
- Depends on: current SexObject/Linen conceptual model
- Notes: architecture plan only, no implementation

9. `REAL_HARDWARE_DAILY_DRIVER_BOOT_PROOF_RUNBOOK_V1` (Risk: Low, Docs-only)
- Depends on: existing hardware proof docs/checklists
- Notes: tighten operational runbook + blocker template

10. `USB_SLOT2_MULTI_HID_ARCH_REVIEW_V1` (Risk: High, Review-only)
- Depends on: current slot2 evidence and deferred USB state
- Notes: strict architecture review, STOP before behavior changes

## Prompt Artifacts
- `/tmp/APP_LAUNCHER_VISUAL_KEYS_HELP_V1.prompt`
- `/tmp/SPINDLE_COMMAND_ALIASES_V1.prompt`
- `/tmp/LINEN_SEARCH_FILTER_KEYBOARD_V1.prompt`
- `/tmp/BELL_EVENT_FILTER_KEYBOARD_V1.prompt`
- `/tmp/ATLAS_THEME_PREVIEW_MARKERS_V1.prompt`
- `/tmp/HANDOFF_INDEX_AND_PROOF_REGISTRY_V1.prompt`
- `/tmp/DAILY_DRIVER_MASTER_GATE_HARDENING_V1.prompt`
- `/tmp/APP_INSTALL_MODEL_PLAN_V1.prompt`
- `/tmp/REAL_HARDWARE_DAILY_DRIVER_BOOT_PROOF_RUNBOOK_V1.prompt`
- `/tmp/USB_SLOT2_MULTI_HID_ARCH_REVIEW_V1.prompt`

## Recommended First 3 To Run
1. `APP_LAUNCHER_VISUAL_KEYS_HELP_V1`
2. `SPINDLE_COMMAND_ALIASES_V1`
3. `LINEN_SEARCH_FILTER_KEYBOARD_V1`

Reason:
- Lowest regression risk first, fast signal, keeps daily-driver gate stable while improving keyboard-first usability.

## STOP FIRST Warnings
- `USB_SLOT2_MULTI_HID_ARCH_REVIEW_V1`
- STOP FIRST before any kernel/ABI/sex-pdx/sexusb behavior changes.
- This mission is architecture review only.

- `APP_INSTALL_MODEL_PLAN_V1`
- STOP FIRST if planning drifts into implementation or ABI/schema edits.
- This mission is docs-only and conceptual.

- `DAILY_DRIVER_MASTER_GATE_HARDENING_V1`
- STOP FIRST before broad script rewrites or gate semantics changes.
- Only minimal diagnostics hardening is allowed.

## Shared Hard Rules Embedded In Every Prompt
- backup before changes
- read handouts/docs first on failure
- rg-first, narrow reads, no broad dumps
- log recurring issues in `docs/handoff`
- no std/libc/threads; no Linux/POSIX assumptions
- PDX-only, no kernel/ABI/sex-pdx edits unless STOP FIRST
- no sexusb/sexinput/pointer unless explicitly hardware/input
- preserve sexdisplay sole framebuffer writer and bounds checks
- no shared-memory/backing-buffer redesign
- no broad refactor
- source-changing missions must run build + daily-driver proof
- no untracked `.bak` artifacts
