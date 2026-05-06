# SPINDLE_DISPLAY_SURFACE_V1

**Date:** 2026-05-06
**Status:** FB properly gated — surface route via silk-shell, no runtime FB access
**Previous:** SPINDLE_LINEN_SPN_V1
**Next:** SPINDLE_COMMAND_SET_V1 (Phase 8)

---

## Summary

Verified and documented the display surface ownership path:
- All framebuffer access is **proof-gated** (inside `INPUT_PROOF_ENABLED` block)
- Normal spawn (PD 12) has **zero runtime FB writes**
- Silk-shell owns Spindle surface (SURFACE_ID_SPINDLE = 0x99)
- Sexdisplay owns final pixel rendering
- Surface marker: `[spindle.fb.proof.disabled] surface=0x99 route=silk-shell fb=gated_proof_only`

---

## Surface Ownership Path

```
Spindle (PD 12)
  ├── Normal runtime: NO framebuffer access (serial-only)
  │   └── Surface rendering: silk-shell internal YARN handler
  │       └── silk-shell → sexdisplay (OP_WINDOW_CREATE, opcodes)
  │
  └── Proof gate (SEXOS_SPINDLE_INPUT_PROOF=1):
      └── Direct WindowBuffer at PFN 0x40000
          └── font::draw_str, draw_pixel, render_scrollback
          └── DEVELOPMENT ONLY — not active in normal spawn
```

---

## FB Access Audit

| Check | Result |
|-------|--------|
| Total FB references in source | 37 |
| FB refs inside proof gate | 20 |
| FB refs outside proof gate | 17 (imports, constants, function defs — never called) |
| Runtime FB writes (normal spawn) | **0** |
| Runtime FB writes (proof gate) | Development only |
| Sexdisplay sole FB writer | **PRESERVED** |

---

## Files Changed

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +1 line — FB gating marker |
| `docs/handoff/SPINDLE_DISPLAY_SURFACE_V1.md` | NEW |

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| entrypoint_build.sh | PASS |
| master_runtime_gate | **GREEN_MASTER** (6/6) |
| Faults | **0** |

### Serial Log

```
[spindle.sexfiles.persist.pending] reason=no_storage_cap
[spindle.bell.pending] reason=no_bell_cap
[spindle.linen.spn.pending] reason=no_linen_cap kind=SpindleSession ext=.spn
[spindle.fb.proof.disabled] surface=0x99 route=silk-shell fb=gated_proof_only
```

---

## Next Prompt

```
SPINDLE_COMMAND_SET_V1
```

Phase 8: Command set finalization (tab completion, history nav, help polish).
