# APP_REGISTRY_STATIC_V2 — Handoff

## Goal
Define one authoritative static app metadata table consumed by Spindle app
commands and app launcher markers.  No cross-PD queries, no live registry sync.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `apps/spindle/src/main.rs` | Static registry proof gate, 8-row table, done marker | +16 |

## Architecture
- **Gate**: `APP_REGISTRY_STATIC_V2_PROOF_ENABLED` via `SEXOS_APP_REGISTRY_STATIC_V2_PROOF=1`
- **Proof**: Emits 8 structured `[app.registry.row]` markers at boot from a static table
- **Fields per row**: id (u8), name (string), sid (surface_id, 0=none), status (PASS/DEFER), launch (active/palette_owned/none)

## Registry Table
| id | Name    | sid | Status | Launch        |
|----|---------|-----|--------|---------------|
| 0  | Spindle | 0   | PASS   | active        |
| 1  | Quil    | 201 | PASS   | palette_owned |
| 2  | Linen   | 200 | PASS   | palette_owned |
| 3  | Bell    | 0   | PASS   | palette_owned |
| 4  | Atlas   | 0   | PASS   | palette_owned |
| 5  | Collar  | 0   | PASS   | palette_owned |
| 6  | Mesh    | 0   | PASS   | palette_owned |
| 7  | Pointer | 0   | DEFER  | none          |

## Markers (serial)
```
[app.registry.row] id=N name=NAME sid=N status=NAME launch=NAME
[app.registry.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_APP_REGISTRY_STATIC_V2_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `app_registry_static`: PASS (8 rows)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No cross-PD queries — static compile-time table only
- ✅ Existing `apps`, `app-info`, `app-status` commands unchanged
- ✅ No new PDX opcodes

## Known Limitations
- Static table — no dynamic app install/uninstall tracking
- No live silk-shell app registry query (PDX opcode needed)
- surface_id only populated for Quil/Linen (200/201); others use 0
- App launcher still gated on kernel spawn + SLOT_SHELL

## Future Follow-up
- Live registry query via new PDX opcode to silk-shell
- Dynamic app install tracking via SexFiles manifest
- surface_id population for Bell/Atlas/Collar/Mesh
