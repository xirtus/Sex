# SHELL_BOOT_SURFACE_ID_ZORDER_FIX_V1

Date: 2026-05-06

## Root Cause
Boot readiness marker code logged a stale second z-order surface id (`SURFACE_ID_MESH` = 202) while the boot-created pair is Quil (201) + Linen (200). This produced a phantom 202 in z-order proof output even though 202 was not part of the boot create chain.

## Findings: 200/201/202
- `SURFACE_ID_LINEN = 200`.
- `SURFACE_ID_QUIL = 201`.
- `SURFACE_ID_MESH = 202` exists as a real optional mesh placeholder surface in shell lifecycle and z-order tables, but it is not part of the two-surface boot readiness pair.
- Boot create markers show only sid 201 + sid 200 in the readiness path.

## Fix Applied
- In boot readiness proof block, changed z-order marker from:
  - first=`SURFACE_ID_QUIL`, second=`SURFACE_ID_MESH`
- To:
  - first=`SURFACE_ID_QUIL`, second=`SURFACE_ID_LINEN`
- Added reject marker when both boot pair surfaces are not visible:
  - `[silk-shell.boot.zorder.reject] reason=boot_pair_not_visible q=<0|1> l=<0|1>`

## Geometry Decision
- No geometry change in this patch.
- Current Quil boot bounds remain `x=100 y=100 w=640 h=480`.
- Current Linen boot bounds remain `x=900 y=500 w=300 h=150`.
- Treat as intentional until a dedicated layout pass defines full-content occupancy policy.

## Build
- `./scripts/entrypoint_build.sh` passes.

## Next Runtime Proof
Verify that boot marker chain now reports z-order with `second=200` (Linen) and no phantom 202 in boot readiness output:
- `[silk-shell.boot.surface.create] sid=201`
- `[silk-shell.boot.surface.create] sid=200`
- `[silk-shell.boot.surface.visible] ...`
- `[silk-shell.boot.zorder] ... first=201 second=200`
- or `[silk-shell.boot.zorder.reject]` if visibility precondition failed
