# SURFACE_ID_REGISTRY_FIX_WEBSTUB_V1

**Status:** PASS IMPLEMENTED — 98/98 gates, 0 faults.
**Date:** 2026-05-16
**Depends on:** `SURFACE_ID_REGISTRY_AUDIT_V1.md`.

---

## Result: PASS — SID collision resolved

WebStub SID changed from 202 (collision with Mesh) to 205 (collision-free).
No surface created. No capability increase.

---

## SID Fix Table

| Field | Old | New |
|-------|-----|-----|
| WebStub SID | 202 (SURFACE_ID_MESH) | 205 (free) |
| Collision | Yes (Mesh) | No |
| Surface | 0 | 0 (deferred) |
| focusable | 0 | 0 |
| launch_exec | 1 | 1 |
| network | 0 | 0 |

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | 5 lines: app_id→SID mapping, review proof, mesh edge, placeholder open |
| `apps/spindle/src/main.rs` | `browser-surface` command text updated (202→205) |

## Proof: 98/98 PASS, 0 faults (no regressions)

## Fault Count: **0**

## Note: No visual surface was created. SID is assigned but surface creation is deferred.

## Commit
```bash
git add servers/silk-shell/src/main.rs apps/spindle/src/main.rs docs/handoff/SURFACE_ID_REGISTRY_FIX_WEBSTUB_V1.md
git commit -m "fix(silk): WebStub SID 202->205 collision resolution"
```
