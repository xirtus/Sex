# Renderer Ownership Audit — Report

**Date:** 2026-05-03
**Audit scope:** servers/sexdisplay/src/main.rs, servers/silk-shell/src/main.rs (display-facing),
crates/silkbar-model/src/lib.rs, docs/handoff/STABLE_BASELINE_20260503.md

---

## 1. Ownership Verdict: PASS — sexdisplay is still a boring pixel-renderer

sexdisplay does **only** what a renderer should:

- **Writes pixels** via bounded render/draw paths only
- **Preserves framebuffer bounds** on every write path
- **Renders from model/state** only (SilkBar model, Surface array, FOCUSED_SURFACE_ID from shell)
- **Avoids shell focus decisions** — FOCUSED_SURFACE_ID is set via OP_SET_FOCUS (0xED) from shell
- **Avoids input policy** — no click/hit-test/button/keyboard dispatch
- **Avoids app lifecycle** — no spawn/kill/terminate/exit logic
- **Avoids direct app internals** — surfaces are opaque IDs
- **Avoids shared-memory/backing-buffer redesign** — PDX scalar args only
- **Validates SilkBar contract** at startup via `validate_silkbar_contract()`
- **Enforces ownership** on 0xEC/0xEB/0xEE/0xEF via `caller_pd` checks

---

## 2. Framebuffer Bounds Verdict: PASS

Every pixel write path has bounds guards:

| Write Path | Bounds Checks |
|---|---|
| `render()` (full frame) | `checked_mul(h)`, `checked_mul(4)`, `checked_add(bytes)`, `idx < total_pixels`, `w/h > MAX_FB_W/H` guard |
| `redraw_top_strip()` (y<50) | Same as render + `h < 51` return guard |
| `redraw_surface_area()` (below bar) | Same as render + `h < 51` return guard |
| `draw_cursor_z_top()` | `ox/oy.max(0)`, `py >= h` break, `px >= w` continue, `idx < total_pixels` |
| `draw_launcher_panel()` | `clamp_surface()` → `py >= h` break, `px >= w` continue, `idx >= total_pixels` continue |
| `composite_pixel()` | `clamp_surface()` called at each call site |
| `handle_primary_fb()` | `w.checked_mul(h)`, `w/h` bounds against MAX_FB_W/MAX_FB_H |

All write paths use the same pattern:
```rust
let idx = y * w + x;
if idx < total_pixels {
    unsafe { core::ptr::write_volatile(fb.add(idx), c); }
}
```

---

## 3. Input/Focus/App Lifecycle Drift Verdict: PASS — no drift

| Concern | sexdisplay | silk-shell |
|---|---|---|
| Input policy | None — no click/hit-test/button dispatch | ✅ Owns click_focus hit-test, drag state, pointer state |
| Focus decisions | Renders z-order from shell-set FOCUSED_SURFACE_ID only | ✅ Owns FOCUS_ID, sends 0xED to display |
| App lifecycle | None — no spawn/kill/terminate | ✅ Shell spawns surfaces, sends create/destroy |
| Panel toggle | Renders panels when active (shell sets position) | ✅ Shell owns toggle_os_panel(), panel state booleans |
| Surface placement | Receives x/y/w/h via 0xEC/0xEB from shell | ✅ Shell computes positions |

---

## 4. Shell/Display Boundary Verdict: PASS

Shell sends these opcodes to display — all are model/state updates, no pixel data:

| Opcode | Shell Sends | Display Receives |
|---|---|---|
| `0xEC` | Surface create (id, pos, size) | Creates surface slot, binds owner_pd |
| `0xEB` | Surface move (id, x, y) | Updates surface position |
| `0xED` | Set focus (id) | Updates FOCUSED_SURFACE_ID for z-order |
| `0xEE` | Surface destroy (id) | Deactivates surface, clears focus if focused |
| `0xEF` | Fill rect (id, rect, color) | Draws solid rect on surface |
| `0xE4` | Window create (x, y, w, h) | Allocates OS surface slot (no owner) |
| `OP_SILKBAR_UPDATE` | (from silkbar) | `apply_update()` on SilkBar model, redraw top strip |

**Shell does NOT write framebuffer.** No pixel/fb/draw/render/composite calls in shell code. The single match is a comment (`// winning composite Pass 1`).

---

## 5. Exact Findings

### PASS findings
| # | Finding | File:Line |
|---|---|---|
| 1 | All framebuffer writes have idx < total_pixels guard | sexdisplay lines 319-320, 358-359, 376-377, 482-483, 513 |
| 2 | Dimensions guarded by MAX_FB_W/MAX_FB_H | sexdisplay lines 282, 336, 370 |
| 3 | `clamp_surface()` bounds surfaces below bar (y>=50) | sexdisplay lines 67-74 |
| 4 | Cursor draw clips: `py >= h` break, `px >= w` continue | sexdisplay lines 475, 480 |
| 5 | Launcher panel uses `clamp_surface()` then per-pixel bounds | sexdisplay line 500 |
| 6 | Ownership validation on all surface mutations (0xEC, 0xEB, 0xEE, 0xEF) | sexdisplay lines 670, 723, 758, 799 |
| 7 | SilkBar contract validated at startup | sexdisplay line 569 |
| 8 | Shell owns focus/input/panel policy; no framebuffer writes | silk-shell: no pixel/fb patterns |
| 9 | `0xDE` legacy opcode safely no-ops | sexdisplay lines 709-713 |
| 10 | `0xE4` window create intentionally skips owner_pd (OS surfaces) | sexdisplay lines 628-653 |

### WARN findings (safe, documented)
| # | Finding | File:Line | Note |
|---|---|---|---|
| 1 | No "render bounds ok" proof marker exists | sexdisplay | Existing `top_strip_render_proof` covers top strip; no marker confirms all bounds paths are active |

### FAIL findings
None.

---

## 6. Smallest Safe Patch

**No FAIL findings — no patch needed for behavior.**

Optional but useful: add a one-time `[sexdisplay.render.bounds.ok]` proof marker in the initial render path to document the bounds guard. This would go in `render()` after the bounds checks pass, before the first pixel write.

Location: `servers/sexdisplay/src/main.rs` around line 301 (after `let total_pixels = pixels;`):
```rust
// Bounds guard proof: confirm all checks passed before first write.
serial_println!("[sexdisplay.render.bounds.ok] w={} h={} total_pixels={}", w, h, total_pixels);
```

This is non-spammy (fires once on first full render), confirms bounds checks passed, and creates a grep-able log marker.

---

## 7. Validation Commands

```bash
# Build
./scripts/entrypoint_build.sh

# Run audit gates
./scripts/audit_invariant_gates.sh

# Verify sexdisplay has no input/focus/lifecycle patterns
rg -n -i "click|button|keyboard|spawn|kill|terminate|lifecycle" \
  servers/sexdisplay/src/main.rs | rg -v "clock|composite_pixel|FOCUS_SURFACE_COLOR|serial_println"

# Verify shell has no framebuffer writes
rg -n -i "fb|pixel|write_volatile|framebuffer|render|draw" \
  servers/silk-shell/src/main.rs | rg -v "// "

# Verify only sexdisplay writes framebuffer (should return 0)
rg -l "write_volatile" servers/*/src/main.rs

# Verify no kernel/sex-pdx edits
git diff -- kernel/ crates/sex-pdx/ crates/silkbar-model/ | wc -l
```

---

## 8. STOP FIRST Conditions

No STOP FIRST conditions triggered. No kernel/sex-pdx/ABI edits needed.

---

## 9. Can Codex Patch Without Kernel/ABI/sex-pdx Edits?

**Yes** — the optional bounds proof marker is a one-line `serial_println!` addition in `servers/sexdisplay/src/main.rs`. No other file changes needed.

---

## Summary

| Dimension | Verdict |
|---|---|
| sexdisplay is renderer-only | ✅ PASS |
| Framebuffer bounds preserved on all paths | ✅ PASS |
| No input policy in display | ✅ PASS |
| No app lifecycle in display | ✅ PASS |
| Shell owns policy, sends updates only | ✅ PASS |
| SilkBar contract validated | ✅ PASS |
| Ownership enforced via caller_pd | ✅ PASS |
| No backing-buffer redesign | ✅ PASS |
| **FAIL findings** | **0** |
| Optional improvement | One-line bounds proof marker |

*End of renderer ownership audit.*
