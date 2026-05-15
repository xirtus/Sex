# OVERNIGHT_PROMPT_SET_V4

## Order
1. APP_REGISTRY_LAUNCH_INTENT_MARKERS_V1
2. SPINDLE_NOTIFY_ALIAS_AUDIT_V1
3. LAUNCHER_STATUS_REASON_NORMALIZE_V1
4. LINEN_QUERY_TOKEN_MARKERS_V1
5. BELL_EMPTY_RING_REASON_AUDIT_V1
6. HANDOFF_INDEX_REFRESH_V3 (docs)
7. PROOF_ENV_REGISTRY_REFRESH_V3 (docs)
8. DAILY_DRIVER_PROOF_LOG_CATALOG_V1 (docs)
9. REAL_HW_BLOCKER_TEMPLATE_PACK_V1 (docs)
10. SLOT2_STOP_FIRST_GATECHECK_V3 (docs)

## Risk
- 1-5 low to low-medium (marker-only)
- 6-10 low (docs-only)

## Dependencies
- Existing shell marker paths in `servers/silk-shell/src/main.rs`
- Existing spindle alias command path in `apps/spindle/src/main.rs`

## Recommended First 4
1. APP_REGISTRY_LAUNCH_INTENT_MARKERS_V1
2. SPINDLE_NOTIFY_ALIAS_AUDIT_V1
3. LAUNCHER_STATUS_REASON_NORMALIZE_V1
4. LINEN_QUERY_TOKEN_MARKERS_V1

## STOP FIRST
- kernel, ABI, sex-pdx, sexusb behavior, sexinput/pointer behavior
