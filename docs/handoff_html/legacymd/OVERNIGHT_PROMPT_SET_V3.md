# OVERNIGHT_PROMPT_SET_V3

## Objective
High-throughput, bounded next-10 mission set with minimal risk and preserved daily-driver proof gate.

## Order, Risk, Dependencies
1. APP_LAUNCHER_KEYS_ROWS_AUDIT_V2 (low) - depends on existing launcher help markers.
2. SPINDLE_APPS_REGISTRY_VIEW_V1 (low) - depends on existing `spindle apps` command path.
3. LINEN_SEARCH_FILTER_MARKERS_V2 (low-medium) - depends on shell-side seeded Linen table access.
4. BELL_SOURCE_FILTER_DETAIL_V2 (low-medium) - depends on local bell ring availability.
5. ATLAS_PREVIEW_APPLY_AUDIT_V2 (low-medium) - depends on atlas key path and preview marker points.
6. HANDOFF_INDEX_REFRESH_V2 (low) - docs-only.
7. PROOF_ENV_REGISTRY_REFRESH_V2 (low) - docs-only.
8. APP_INSTALL_MODEL_PHASEB_PLAN_V1 (low) - docs-only architecture.
9. REAL_HW_DAILY_DRIVER_RUNBOOK_V2 (low) - docs-only operator flow.
10. SLOT2_MULTI_HID_STOP_FIRST_REVIEW_V2 (low) - docs-only, explicit no-impl boundary.

## Recommended First 4
1. APP_LAUNCHER_KEYS_ROWS_AUDIT_V2
2. SPINDLE_APPS_REGISTRY_VIEW_V1
3. LINEN_SEARCH_FILTER_MARKERS_V2
4. BELL_SOURCE_FILTER_DETAIL_V2

## STOP FIRST Warnings
- Any mission requiring kernel edits: STOP and handoff.
- Any mission requiring ABI/sex-pdx changes: STOP and handoff.
- Any mission requiring sexusb behavior change in this batch: STOP and handoff.
- Any pointer/sexinput work outside docs review: STOP and handoff.

## Prompt Files
- /tmp/OVERNIGHT_PROMPT_SET_V3_PLAN.prompt
- /tmp/APP_LAUNCHER_KEYS_ROWS_AUDIT_V2.prompt
- /tmp/SPINDLE_APPS_REGISTRY_VIEW_V1.prompt
- /tmp/LINEN_SEARCH_FILTER_MARKERS_V2.prompt
- /tmp/BELL_SOURCE_FILTER_DETAIL_V2.prompt
- /tmp/ATLAS_PREVIEW_APPLY_AUDIT_V2.prompt
- /tmp/HANDOFF_INDEX_REFRESH_V2.prompt
- /tmp/PROOF_ENV_REGISTRY_REFRESH_V2.prompt
- /tmp/APP_INSTALL_MODEL_PHASEB_PLAN_V1.prompt
- /tmp/REAL_HW_DAILY_DRIVER_RUNBOOK_V2.prompt
- /tmp/SLOT2_MULTI_HID_STOP_FIRST_REVIEW_V2.prompt
