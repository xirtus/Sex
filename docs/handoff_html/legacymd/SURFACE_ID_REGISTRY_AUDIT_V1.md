# SURFACE_ID_REGISTRY_AUDIT_V1

**Status:** PASS REVIEW ONLY — No source edits.
**Date:** 2026-05-16
**Next:** `SURFACE_ID_REGISTRY_FIX_WEBSTUB_V1.md`.

---

## 0. PASS REVIEW ONLY — 1 collision found, no renumbering

---

## 1. Canonical Surface ID Table

### Core Shell / Overlay Surfaces (0x90–0x99 range)

| SID (hex) | SID (dec) | Constant | Owner | Focusable | Notes |
|-----------|-----------|----------|-------|-----------|-------|
| 0x90 | 144 | CURSOR_SURFACE_ID | Cursor | No | Cursor overlay (sexdisplay) |
| 0x92 | 146 | SURFACE_ID_LAUNCHER | Launcher | Overlay | Launcher panel (sexdisplay) |
| 0x96 | 150 | SURFACE_ID_SCENE_SETTINGS | Scene Settings | Overlay | Scene settings panel |
| 0x98 | 152 | SURFACE_ID_COMMAND_PALETTE | Command Palette | Overlay | Toggled by backtick |
| 0x99 | 153 | SURFACE_ID_SPINDLE | Spindle | Yes | Terminal console |

### App / Test Surfaces (100–103)

| SID | Constant | Owner | Focusable | Notes |
|-----|----------|-------|-----------|-------|
| 100 | SURFACE_ID_APP | Generic App | Yes | Generic app surface |
| 101 | SURFACE_ID_STATIC | Static | — | Static surface |
| 102 | SURFACE_ID_TEST3 | Test 3 | — | Test surface |
| 103 | SURFACE_ID_TEST4 | Test 4 | — | Test surface |

### System Tool / Placeholder Surfaces (200–204)

| SID | Constant | Owner | Focusable | Launch | Notes |
|-----|----------|-------|-----------|--------|-------|
| 200 | SURFACE_ID_LINEN | Linen | Yes | launch_exec=1 | Document editor |
| 201 | SURFACE_ID_QUIL | Quil | Yes | launch_exec=1 | Text editor |
| **202** | **SURFACE_ID_MESH** | **Mesh** | Overlay | N/A | **COLLISION** |
| 203 | SURFACE_ID_COLLAR | Collar | Overlay | N/A | Future authority UI |
| 204 | SURFACE_ID_BELL_PLACEHOLDER | Bell | Overlay | N/A | Bell notification placeholder |

### Unassigned (no constant, intent only)

| SID | Intended Owner | Actual Owner | Status |
|-----|---------------|-------------|--------|
| **202** | **WebStub/Browser** | **Mesh** | **COLLISION** |
| — | Atlas | None (no surface) | No SID, nonfocusable overlay |
| — | WebStub | Collides with Mesh | Needs own SID |

---

## 2. Collision: SID 202 → Mesh + WebStub

```
Source A: pub const SURFACE_ID_MESH: u64 = 202;
  → Mesh has a live placeholder surface at SID 202
  → Position tracking: SURFACE_202_X/Y/W/H
  → Object links referencing surface 202

Source B: app_id=7 => 202; // WebStub placeholder
  → Spindle sends SLOT_SHELL launch for app_id=7
  → Shell maps app_id=7 to SID 202
  → Calls open_app_in_active_scene_by_sid(202)
  → Opens Mesh's surface, not WebStub's
  → Result: honest no-op (no_surface_placeholder_only)
```

**Impact**: WebStub cannot have its own surface. Launch opens Mesh instead. The system tolerates this gracefully (marker: `sid_202_no_surface`).

**Severity**: Low. No crash, no data corruption. Only cosmetic — WebStub has no visible surface.

---

## 3. Recommended SID Ranges (future convention)

| Range | Purpose | Current Occupants |
|-------|---------|-------------------|
| 0x80–0x8F | Reserved (kernel/fb) | — |
| 0x90–0x9F | Shell overlays | Cursor, Launcher, SceneSettings, CommandPalette, Spindle |
| 100–149 | User apps | APP, STATIC, TEST3, TEST4 |
| 150–199 | Editors/tools | (reserved for future) |
| 200–204 | System placeholders | Linen, Quil, Mesh, Collar, Bell |
| **205** | **Recommended: WebStub** | **Free** |
| 206–249 | Future system surfaces | Free |
| 250–299 | Deferred placeholders | Free |

---

## 4. Recommended WebStub SID: **205**

- Next available after the 200–204 block
- No collision risk
- Consistent with system placeholder range (200+)
- Single-line change: `7 => 205` at line 17008 in silk-shell
- No kernel/ABI/sex-pdx changes

---

## 5. Risks / Blockers

| Risk | Mitigation |
|------|-----------|
| Shell assumes SID 202=Mesh in hardcoded paths | Audit all `SURFACE_ID_MESH` references before assigning 205 to WebStub |
| sexdisplay needs new surface for 205 | Uses existing 0xEC upsert — no protocol change |
| Golden hash may change if WebStub surface adds pixels to top strip | Re-capture golden hash after surface creation |
| Other deferred apps may need SIDs later | Reserve 206+ in this audit |

---

## 6. Next Implementation Prompt

**MISSION: SURFACE_ID_REGISTRY_FIX_WEBSTUB_V1**

- Change `app_id=7 => 202` to `app_id=7 => 205` in shell
- Add `SURFACE_ID_WEBSTUB: u64 = 205` constant
- Update all WebStub/browser markers from sid=202 to sid=205
- Optionally create a placeholder surface for WebStub at SID 205 (same pattern as Bell/Mesh)
- Keep network=0 engine=0 focusable=0
- Build + daily proof must pass
- STOP FIRST if any other SID collision found

---

## 7. STOP FIRST Boundaries (all pass for review)

| Boundary | Status |
|----------|--------|
| SID values are global ABI | ❌ No — shell-local constants only |
| SID values are persisted/durable | ❌ No — volatile only |
| SID values require sex-pdx/kernel edits | ❌ No — shell-local |
| Broad registry refactor needed | ❌ No — single constant + mapping change |

---

## 8. Handoff Path

```
docs/handoff/SURFACE_ID_REGISTRY_AUDIT_V1.md
```

---

## 9. Commit

```bash
git add docs/handoff/SURFACE_ID_REGISTRY_AUDIT_V1.md
git commit -m "docs(silk): surface ID registry audit V1"
```
