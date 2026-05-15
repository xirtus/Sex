# OVERNIGHT_PROMPT_SET_V2

## Objective
Maximize tonight's throughput by executing 10 mission intents via 4 batch prompts, reducing repeated build/gate cycles.

## Batch Layout (10 intents -> 4 prompts)

### Batch A: shell/app polish (3 intents)
- APP_LAUNCHER_VISUAL_KEYS_HELP_V1
- SPINDLE_COMMAND_ALIASES_V1
- LINEN_SEARCH_FILTER_KEYBOARD_V1
Prompt: `/tmp/OVERNIGHT_BATCH_A_SHELL_APP_POLISH_V2.prompt`

### Batch B: event/theme polish + gate check (3 intents)
- BELL_EVENT_FILTER_KEYBOARD_V1
- ATLAS_THEME_PREVIEW_MARKERS_V1
- DAILY_DRIVER_MASTER_GATE_HARDENING_V1 (minimal/no-op hardening)
Prompt: `/tmp/OVERNIGHT_BATCH_B_EVENT_THEME_POLISH_V2.prompt`

### Batch C: docs operations (2 intents + consistency tie-in)
- HANDOFF_INDEX_AND_PROOF_REGISTRY_V1
- REAL_HARDWARE_DAILY_DRIVER_BOOT_PROOF_RUNBOOK_V1
Prompt: `/tmp/OVERNIGHT_BATCH_C_DOCS_OPERATIONS_V2.prompt`

### Batch D: architecture depth (2 intents + bridge)
- APP_INSTALL_MODEL_PLAN_V1
- USB_SLOT2_MULTI_HID_ARCH_REVIEW_V1
Prompt: `/tmp/OVERNIGHT_BATCH_D_ARCH_DEPTH_V2.prompt`

## Throughput Strategy
- One backup snapshot per batch.
- One build + one daily-driver proof cycle for code-changing batches (A/B).
- Docs-only validation grep checks for C/D.
- Commit + push by default on PASS.

## Global Guardrails
- Preserve 18/18 PASS, faults=0 baseline.
- STOP FIRST before any kernel/ABI/sex-pdx/sexusb/pointer behavior changes.
- Stop after 2 repeated failures on same issue.
- Never leave `*.bak-*` artifacts.

## Suggested Run Order Tonight
1. Batch A
2. Batch B
3. Batch C
4. Batch D

## Expected Deliverables
- 4 executed batch reports
- updated mission docs
- pushed checkpoints after each PASS batch
- one final summary of completed intents, blockers, and next narrow mission
