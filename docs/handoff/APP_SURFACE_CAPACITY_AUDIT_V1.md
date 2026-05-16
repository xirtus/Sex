# APP_SURFACE_CAPACITY_AUDIT_V1

**Status:** PASS REVIEW ONLY — Capacity exists, expansion is safe.
**Date:** 2026-05-16
**Next:** `APP_SURFACE_CAPACITY_EXPAND_WEBSTUB_V1.md`.

---

## 0. PASS REVIEW ONLY — 1 free frame slot, array expansion safe

---

## 1. Current APP_SURFACES Table (7 entries)

| Index | App | SID | Frame ID | Focusable | Boot (x,y,w,h) |
|-------|-----|-----|----------|-----------|-----------------|
| 0 | Linen | 200 | 2 | Yes | (900,500,300,150) |
| 1 | Quil | 201 | 3 | Yes | (100,100,640,480) |
| 2 | Mesh | 202 | 4 | Yes | (200,100,640,480) |
| 3 | Collar | 203 | 5 | Yes | (300,100,640,480) |
| 4 | Bell | 204 | 6 | Yes | (400,100,640,480) |
| 5 | CmdPalette | 0x98 | 7 | Yes | (400,200,480,240) |
| 6 | Spindle | 0x99 | 9 | Yes | (200,200,500,300) |

## Frame ID Map

| Frame ID | Owner | Used? |
|----------|-------|-------|
| 0 | Spindle (self-hosted, not in APP_SURFACES frame_id) | Special |
| 1 | — | **FREE** |
| 2 | Linen | Used |
| 3 | Quil | Used |
| 4 | Mesh | Used |
| 5 | Collar | Used |
| 6 | Bell | Used |
| 7 | Command Palette | Used |
| **8** | — | **FREE** |
| 9 | Spindle | Used |

---

## 2. Minimum WebStub Expansion Plan

### What's needed:

| Item | Value |
|------|-------|
| SID | 205 (already assigned) |
| Frame ID | **8** (free slot) |
| Boot geometry | (500,100,400,300) — tiled right of Bell |
| APP_SURFACES | `[AppSurfaceSpec; 7]` → `[AppSurfaceSpec; 8]` |
| focusable | 1 (once surface exists) |
| closeable | 0 (same as all system apps) |
| Position tracking | `SURFACE_205_X/Y/W/H` (same pattern as 200-204) |
| Frame registration | Uses existing `register_app_surfaces()` path |

### Changes required:

1. **Constants** (+5 lines):
   ```rust
   const BROWSER_FRAME_ID: u32 = 8;
   const BROWSER_BOOT_X: i32 = 500;
   const BROWSER_BOOT_Y: i32 = 100;
   const BROWSER_BOOT_W: u32 = 400;
   const BROWSER_BOOT_H: u32 = 300;
   ```

2. **APP_SURFACES array** (change `7` to `8`, +1 entry):
   ```rust
   const APP_SURFACES: [AppSurfaceSpec; 8] = [ ...existing 7..., browser_entry ];
   ```

3. **Position tracking** (+4 statics):
   ```rust
   static mut SURFACE_205_X: i32 = BROWSER_BOOT_X;
   static mut SURFACE_205_Y: i32 = BROWSER_BOOT_Y;
   static mut SURFACE_205_W: u32 = BROWSER_BOOT_W;
   static mut SURFACE_205_H: u32 = BROWSER_BOOT_H;
   ```

4. **SID bounds match** — existing `match` on SID for position lookup needs `SURFACE_ID_BROWSER => ...`

5. **No sexdisplay changes** — existing 0xEC upsert handles any surface ID.

6. **No protocol changes** — same IPC path as Mesh/Bell.

### Expected impact:

| Item | Impact |
|------|--------|
| APP_SURFACES.len() | 7 → 8 |
| MAX_FRAMES | 9 (no change, frame 8 fits) |
| Golden hash | **Will change** (new frame adds top-strip pixels) |
| Gate count | 98 → 99 (new gate for browser surface) |
| shell binary size | ~200 bytes (constants + array entry) |

---

## 3. Risk Classification

| Risk | Level | Mitigation |
|------|-------|-----------|
| Array expansion local | **Low** — `[7]`→`[8]` is a local constant change |
| Frame model consistency | **Low** — Frame 8 fits in MAX_FRAMES=9 |
| Renderer bounds | **Low** — Uses existing 0xEC path |
| Golden hash change | **Medium** — Must re-capture after surface creation |
| App registry truth | **Low** — Same pattern as Mesh/Bell |
| Spindle command truth | **Low** — Browser-surface command already exists |
| ATLAS_MAX_FRAMES_PER_SCENE | **Low** — 9, frame 8 fits |
| Duplicate validation | **Low** — `APP_SURFACES` validates at boot for dupes |

---

## 4. STOP FIRST Boundaries (all pass)

| Boundary | Status |
|----------|--------|
| APP_SURFACES tied to ABI | ❌ No — shell-local |
| Unsafe indexing without bounds | ❌ No — uses `.len()` and `find()` |
| Frame constants require renderer redesign | ❌ No — existing 0xEC path |
| Golden hash changes unexpectedly | ⚠️ Expected — re-capture after |
| Broad app registry refactor | ❌ No — same registration path |

---

## 5. Recommended Next Prompt

**MISSION: APP_SURFACE_CAPACITY_EXPAND_WEBSTUB_V1**

- Add `SURFACE_ID_BROWSER = 205` and `BROWSER_FRAME_ID = 8`
- Expand `APP_SURFACES` from `[7]` to `[8]` with WebStub entry
- Add `SURFACE_205_X/Y/W/H` position tracking
- Wire SID 205 into position lookup match
- Update `browser.placeholder.truth` markers (focusable=1, surface=1, rendered=1 if sexdisplay renders)
- Update Spindle commands
- Re-capture golden hash
- Build + daily proof must pass
- STOP FIRST if any SID/frame collision found

---

## 6. Handoff

```
docs/handoff/APP_SURFACE_CAPACITY_AUDIT_V1.md
```

## 7. Commit

```bash
git add docs/handoff/APP_SURFACE_CAPACITY_AUDIT_V1.md
git commit -m "docs(silk): APP_SURFACES capacity audit V1"
```
