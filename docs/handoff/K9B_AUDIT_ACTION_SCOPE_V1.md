# K9b: Audit Action Scope Closure

**Status:** Complete
**Date:** 2026-05-05
**Purpose:** Verify K9 resolved the K8 global-debug-trigger risk by scoping PrintScreen
(0x59) to Linen focus, matching J/K selection gating. Docs only.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║                    PASS_K9_SCOPE                             ║
╠══════════════════════════════════════════════════════════════╣
║ K8 global-trigger risk: RESOLVED                             ║
║ All 3 Linen keyboard triggers gated identically              ║
║ Success chain unchanged when Linen focused                   ║
║ Reject path explicit when not focused                        ║
║ Forbidden areas: CLEAN                                       ║
║ Ready for next feature work                                  ║
╚══════════════════════════════════════════════════════════════╝
```

**Verdict: PASS_K9_SCOPE**

## K8 Risk Closure

| K8 Risk | Severity | Status | Resolution |
|---------|----------|--------|------------|
| PrintScreen global debug trigger | LOW | ✅ RESOLVED | Scoped to `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN` in K9 commit 3973fcd |

One remaining K8 risk item resolved. All three Linen keyboard triggers now share identical scoping.

## Trigger Gating Table

| Trigger | Scancode | Action | Gate | Before K9 | After K9 |
|---------|----------|--------|------|-----------|----------|
| J | 0x24 | SelectNextLinenObject | `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN` | ✅ Gated | ✅ Gated |
| K | 0x25 | SelectPrevLinenObject | `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN` | ✅ Gated | ✅ Gated |
| PrintScreen | 0x59 | OpenObjectInQuil | `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN` | ❌ Global | ✅ Gated |

**All three now consistent.** No global debug triggers remain for the Linen selection flow.

## Success Marker Chain Unchanged

When Linen IS focused, PrintScreen follows the exact same marker chain as before K9:

```
[linen.object_select.current] id=N
[linen.quil.open.request] id=N
[collar.gate.check] → [collar.gate.allow_stub]
[linen.quil.open.dynamic_id] / [linen.quil.open.reuse_existing]
[linen.quil.buffer.linked]
[mesh.object_link.start/row/done]
[bell.event.stub/object_link/done]
[quil.buffer_list.render/row/done]
```

Zero changes to J4/J5/J6/J7/K3 internals. The guard is purely at the SurfaceAction dispatch level.

## Reject Marker Proof

When Linen is NOT focused, each trigger emits a clear reject:

| Trigger | Reject Marker | Line |
|---------|--------------|------|
| PrintScreen | `[linen.quil.open.reject] reason=not_focused` | 8459 |
| J | `[linen.object_select.reject] reason=not_focused` | 8469 |
| K | `[linen.object_select.reject] reason=not_focused` | 8481 |

All three produce a consistent `reason=not_focused` proof marker. No silent failures.

## Forbidden-Area Check

| Area | Status |
|------|--------|
| `kernel/` | ✅ CLEAN |
| `crates/sex-pdx/` | ✅ CLEAN |
| `servers/sexdisplay/` | ✅ CLEAN |
| `servers/linen/` (real server) | ✅ CLEAN |
| `servers/quil/` (real server) | ✅ CLEAN |
| PDX ABI/opcodes | ✅ CLEAN — 0x59, 0x24, 0x25 are shell-local scancode mappings |
| Lifecycle/tombstone | ✅ CLEAN — no changes |

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| Bell placeholder naming (204 vs 0x95) | LOW | Deferred — documented |
| Seed pre-links coherent but not J5/J7-runtime-created | LOW | K2C boot sync |
| No real Collar authorization (grant_ref=0) | MEDIUM | STOP FIRST for real |
| Single 0xEF fill rect limits per-row visuals | MEDIUM | Requires sexdisplay multi-rect (STOP FIRST) |

No new risks introduced. One risk removed (PrintScreen global trigger).

## Next Safest Step

**Command palette stub design** — new placeholder surface following the I1-I3 (Mesh/Collar/Bell)
pattern. A command palette provides softkey/command input without opening a new subsystem.
Docs-only design first.

Alternatively: **K10 rapid audit K8-K9b** for a clean milestone close-out before the
command palette.
