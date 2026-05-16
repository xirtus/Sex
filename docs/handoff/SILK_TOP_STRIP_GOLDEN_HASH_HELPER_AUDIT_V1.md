# SILK_TOP_STRIP_GOLDEN_HASH_HELPER_AUDIT_V1

**Status:** PASS REVIEW ONLY — Golden hash harness already exists.
**Date:** 2026-05-16
**Depends on:** `SILK_TOP_STRIP_GOLDEN_HASH_PLAN_V1.md`.
**Next:** Add golden comparison gate (Phase 3 of the plan).

---

## 0. PASS REVIEW ONLY / STOP FIRST

**PASS REVIEW ONLY** — No STOP FIRST triggered. The golden hash harness is already implemented and running in every boot.

---

## 1. Safety Verdict

**SAFE to extend.** The existing harness requires zero architectural changes. Only a golden comparison constant + gate logic needs adding.

---

## 2. Helper Audit Table

| Helper | Deterministic? | Notes |
|--------|---------------|-------|
| `top_strip_render_proof()` | ✅ Yes | Already hashes top 50 rows with FNV-1a. Runs once per boot after first live render. |
| `redraw_top_strip()` | ✅ Yes | Redraws bar background, clock, chips, bell, frame edges. Called on each surface update cycle. |
| `fill_rect_color()` | ✅ Yes | Pure lookup from static surface fill-rect array. No randomness. |
| `glass_over_bg()` | ✅ Yes | Static pattern computation from x,y coordinates. No randomness. |
| `alpha_blend_xrgb_over_xrgb()` | ✅ Yes | Pure arithmetic on u32 ARGB values. |
| `bar_color()` | ✅ Yes | Static color based on bar state fields. |
| `workspace_color()` | ✅ Yes | Static per-pixel workspace chip color lookup. |
| `chip_color()` | ✅ Yes | Static per-pixel chip color lookup. |
| `clock_fg_at()` | ✅ Yes | Font glyph pixel lookup from fixed bitmap. |
| `bell_badge_at()` | ✅ Yes | Font digit pixel lookup from fixed bitmap. |
| `render()` | ✅ Yes | Full framebuffer render. Called before hash capture. |
| `pulse_alpha()` | ⚠️ Uses frame counter | Deterministic per-frame but varies across frames. Not in top-strip hash region. |

### Key Findings

1. **`top_strip_render_proof()` already exists** at line 1361 of `servers/sexdisplay/src/main.rs`. It:
   - Validates FB bounds (HIGH_HALF_BASE, MAX_FB_W, 50-row minimum)
   - Hashes the first 50 rows (strip_rows=50) using FNV-1a
   - Prints the hash: `[silk.render_proof.top_strip.hash] value=0x...`
   - Reports `ok` if any pixel is non-zero, `fail` if all-zero

2. **Live hash observed** in daily proof boot:
   ```
   [silk.render_proof.top_strip.hash] value=0xfd6093ac9ade7b4d
   ```

3. **No golden comparison yet.** The hash is printed but never compared to an expected constant. Adding a `const GOLDEN_TOP_STRIP_HASH: u64 = 0xfd6093ac9ade7b4d;` and a pass/fail gate is the only missing piece.

4. **Hash stability caveat**: The hash covers 50 rows which includes the frame top bar (28px) + rim (4px) + surface content below. If surface content or frame sizes change, the hash will change. The golden hash must be re-captured after any approved visual change.

---

## 3. Recommended Route: **A — Reuse existing helpers**

The infrastructure is already built. Only a comparison gate is needed.

### Proposed next implementation (Phase 3):

1. Add `const GOLDEN_TOP_STRIP_HASH: u64 = 0xfd6093ac9ade7b4d;` in sexdisplay
2. After computing the hash in `top_strip_render_proof()`, compare to golden
3. Emit `[silk.topstrip.hash.result] hash=... golden=... ok=1/0`
4. Add `gate_top_strip_hash` to daily driver master gate
5. Gate requires hash match: if mismatch, gate FAILS (not SKIP)

### Why not other routes:

| Route | Veto Reason |
|-------|------------|
| B (duplicated primitives) | Unnecessary — existing helpers are already deterministic. |
| C (script/log-hash only) | Weaker — in-code comparison gives immediate failing gate on regression. |
| D (STOP FIRST) | Not triggered — no blockers found. |

---

## 4. Implementation Blockers

**None.** All helpers are deterministic. No architectural changes needed.

### Risks to monitor:

| Risk | Mitigation |
|------|-----------|
| Hash changes after approved visual change | Re-capture golden hash; document in commit message. |
| Hash instability across boots | Hash is FNV-1a over raw u32 pixels; deterministic if render is deterministic. |
| Frame count/size changes | Golden hash must match the current frame layout (3 frames, specific sizes). |

---

## 5. STOP FIRST Boundaries (all pass)

| Boundary | Status |
|----------|--------|
| Renderer architecture refactor | ❌ Not needed — harness exists |
| Framebuffer ownership change | ❌ Not needed — reads live FB, no new writes |
| New ABI/protocol | ❌ Not needed — local hash computation |
| Heap/std/libc/thread dependency | ❌ Not needed — stack-only u64 + loop |
| Removing/weakening bounds checks | ❌ Not needed — bounds validated before hash |
| Fake hardcoded hash | ❌ Will use actual hash from golden boot |
| QEMU GUI dependence | ❌ Hash computed in-guest, logged via serial |
| Broad sexdisplay rewrite | ❌ Not needed — single function modification |

---

## 6. Next Prompt Draft Summary

**MISSION: SILK_TOP_STRIP_GOLDEN_HASH_GATE_V1**

Implement Phase 3: golden hash comparison gate.
- Add `GOLDEN_TOP_STRIP_HASH = 0xfd6093ac9ade7b4d` constant.
- Compare runtime hash to golden in `top_strip_render_proof()`.
- Emit pass/fail marker: `[silk.topstrip.hash.result]`.
- Add `gate_top_strip_hash` to daily driver master gate.
- Build + daily proof must pass (hash matches golden).
- STOP FIRST if hash doesn't match (requires re-capture in a clean boot).

---

## 7. Handoff Path

```
docs/handoff/SILK_TOP_STRIP_GOLDEN_HASH_HELPER_AUDIT_V1.md
```

---

## 8. Commit Command

```bash
git add docs/handoff/SILK_TOP_STRIP_GOLDEN_HASH_HELPER_AUDIT_V1.md
git commit -m "docs(silk): top-strip golden hash helper audit V1"
```

---

*End of SILK_TOP_STRIP_GOLDEN_HASH_HELPER_AUDIT_V1.md*
