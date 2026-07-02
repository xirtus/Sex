# REAL_HARDWARE_DAILY_DRIVER_BOOT_PROOF_RUNBOOK_V1

Status: docs-only runbook refinement

## Goal
Tighten real-hardware daily-driver boot proof workflow with a short, repeatable checklist and a strict blocker template.

## Baseline
- Expected QEMU baseline before hardware attempt: `18/18 PASS`, `faults=0`
- Keyboard-first V1 proven
- SilkBar ABI Phase 1-5 proven
- Pointer/USB slot2 remains deferred

## Quick Run Checklist (Operator)

### 0) Preflight (mandatory)
1. `git status --short`
2. Confirm no unexpected source edits before hardware boot.
3. `./scripts/run_daily_driver_proof.sh /tmp/sexos_pre_hw_daily_driver.log`
4. Confirm in output:
- `PASS gates: 18`
- `FAIL gates: 0`
- `FINAL: PASS (18 gates proved, 0 skipped, 0 faults)`

If preflight fails, STOP. Do not boot hardware.

### 1) Prepare Boot Media
1. Confirm fresh ISO exists: `sexos-v1.0.0.iso`
2. Copy ISO to Ventoy media root.
3. Sync/unmount media cleanly.

### 2) Hardware Boot Steps
1. Enter firmware boot menu.
2. Select Ventoy USB boot.
3. Select `sexos-v1.0.0.iso`.
4. Wait for desktop + SilkBar visual readiness.

### 3) Manual Keyboard Proof Steps
1. Toggle palette with backtick.
2. Verify row navigation with `J/K`.
3. Launch at least Spindle/Linen/Quil/Bell/Atlas/Collar/Mesh once.
4. Verify no freeze/panic during rapid palette open/close and scene toggles.
5. Hold idle 60s; confirm clock/status still updating.

## Evidence Capture Requirements
For each hardware run record:
- Date/time
- Commit hash
- ISO name
- Host model / BIOS mode (UEFI/Legacy)
- Pass/fail of each manual proof step
- Photos/video for any failure state

## Failure Blocker Template
Use this exact template in a new handoff file:
`docs/handoff/HARDWARE_BLOCKER_<YYYYMMDD>_<short_slug>.md`

Template:

```
# HARDWARE_BLOCKER_<YYYYMMDD>_<short_slug>

## Symptom
- One-line failure description

## Last Known Good
- commit:
- ISO:
- preflight gate result:

## Exact Evidence
- observed screen state:
- marker/log snippets (if any):
- photo/video references:

## Repro Steps
1.
2.
3.

## Scope Boundaries (STOP FIRST)
- no kernel edits performed: yes/no
- no sex-pdx/ABI edits performed: yes/no
- no sexusb/sexinput behavior edits performed: yes/no

## Next Narrow Diagnostic Action
- one bounded step only
```

## STOP FIRST Rules
STOP and escalate before any of the following:
- kernel/ABI/sex-pdx changes
- sexusb/sexinput behavior changes
- display protocol or framebuffer ownership changes
- “quick-fix” source edits done directly from hardware failures without QEMU repro

## Non-goals
- No source changes in this mission
- No runtime behavior changes
- No new USB/pointer implementation work

## Related Docs
- `docs/handoff/REAL_HARDWARE_DAILY_DRIVER_BOOT_PROOF_V1.md`
- `docs/handoff/OVERNIGHT_PLANS_AND_PROMPTS_V1.md`
- `docs/handoff/USB_SLOT2_MOUSE_BLOCKED_HANDOFF_V1.md`
