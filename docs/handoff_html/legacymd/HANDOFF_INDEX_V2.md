# HANDOFF_INDEX_V2

## Current Proven Baseline
- Daily-driver proof profile: `./scripts/run_daily_driver_proof.sh`
- Expected result: `18/18 PASS`, `faults=0`
- Keyboard-first daily-driver V1: proven
- SilkBar ABI phases 1-5: proven

## Active Keyboard-First Mission Track
- `APP_LAUNCHER_KEYS_ROWS_AUDIT_V2` (done)
- `SPINDLE_APPS_REGISTRY_VIEW_V1` (done)
- `LINEN_SEARCH_FILTER_MARKERS_V2` (done)
- `BELL_SOURCE_FILTER_DETAIL_V2` (done)
- `ATLAS_PREVIEW_APPLY_AUDIT_V2` (done)

## Proof and Gate Docs
- `DAILY_DRIVER_PROOF_PROFILE_V1.md`
- `DAILY_DRIVER_MASTER_GATE_HARDENING_V1.md`
- `PROOF_ENV_REGISTRY_V1.md`
- `PROOF_ENV_REGISTRY_V2.md`

## Architecture and Deferred Docs
- `APP_INSTALL_MODEL_PLAN_V1.md`
- `APP_INSTALL_MODEL_PHASEB_PLAN_V1.md`
- `REAL_HARDWARE_DAILY_DRIVER_BOOT_PROOF_RUNBOOK_V1.md`
- `REAL_HW_DAILY_DRIVER_RUNBOOK_V2.md`
- `USB_SLOT2_MULTI_HID_ARCH_REVIEW_V1.md`
- `SLOT2_MULTI_HID_STOP_FIRST_REVIEW_V2.md`

## STOP FIRST Topics
- Any kernel change
- Any `sex-pdx`/ABI contract change
- Any `sexusb` behavior change for slot2 HID
- Any `sexinput`/pointer behavior change
- Any framebuffer writer ownership change (must remain sexdisplay-only)

## Deferred Blockers
- Slot2 multi-HID runtime behavior changes (requires STOP FIRST review)
- USB pointer/tablet implementation waves (deferred)
- Any kernel/ABI transport refactor tied to HID expansion (deferred)

## Recommended Continuation Order
1. Keep shell/app marker polishing in bounded proof gates.
2. Keep docs/runbook updates synchronized with every proof milestone.
3. Gate every source change with build + daily-driver proof.
