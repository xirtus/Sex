# QUIL_RUNTIME_GEOMETRY_CONTRACT_PLAN_V1

**Status:** Design — No Implementation
**Date:** 2026-05-06
**Purpose:** Design the safest way for Quil to know its panel-local drawable bounds
without breaking PDX/MPK/no_std rules. Docs only.

---

## 1. Audit: Shell-Owned Surface Bounds vs Quil Internal Draw Constants

This document distinguishes two separate quantities that are often conflated:

| Quantity | Owner | Where stored | Changes at runtime? |
|----------|-------|-------------|---------------------|
| **Surface bounds** — the (x, y, w, h) of Quil's panel on the compositor | **Shell** (silk-shell) | sexdisplay surface registry (via 0xEC); shell's `SURFACE_201_*` shadow vars | Yes — shell updates on tile/re-tile |
| **Draw constants** — the (x, y, w, h) Quil uses in its 0xEF fill rect calls | **Quil** (quil server) | hardcoded in quil/src/main.rs (e.g. 2000x2000 oversize) | Never — Quil has no bounds contract yet |

### Shell-owned surface bounds for Quil

The shell stores Quil surface geometry in mutable static shadow variables:

| Variable | Boot default | File:line |
|----------|-------------|-----------|
| `SURFACE_201_X` | 100 | `silk-shell/src/main.rs:3843` |
| `SURFACE_201_Y` | 100 | `silk-shell/src/main.rs:3844` |
| `SURFACE_201_W` | 640 | `silk-shell/src/main.rs:3845` |
| `SURFACE_201_H` | 480 | `silk-shell/src/main.rs:3846` |

These boot defaults are **shell-side legacy constants**. After
`SHELL_BOOT_TILED_DEMO_LAYOUT_V1`, the shell may assign tiled geometry
that differs from these defaults. The authoritative surface bounds are
whatever the shell sends via 0xEC at runtime, not the boot defaults.

Quil **never reads** these variables. They are shell-internal shadows.

### How shell surface bounds reach sexdisplay

The shell sends Quil surface geometry to sexdisplay via **0xEC** (OP_SURFACE_CREATE_ID):

- Boot: `0xEC sid=201 x=<boot_x> y=<boot_y> w=<boot_w> h=<boot_h>`
- Tile/re-tile: `0xEC sid=201 x=<rx> y=<ry> w=<rw> h=<rh>` (in `tile_visible_frames`)

**0xEC is a one-shot upsert; sexdisplay stores the geometry but never echoes it back.**

### How shell surface bounds reach Quil

**They do not.** Quil receives only:

| Opcode | Meaning | From |
|--------|---------|------|
| `OP_QUIL_PING` (0xF0) | Route verification ping | shell |
| `OP_HID_EVENT` (0x202) | Forwarded keyboard scancode | shell (via sexinput) |

**There is zero coupling between shell-assigned surface bounds and Quil's
draw constants.** Quil draws an oversized 2000x2000 fill rect that relies
entirely on sexdisplay's surface-local clamping to avoid OOB writes. This
works because sexdisplay clamps, not because Quil knows its bounds.

---

## 2. Audit: Existing PDX Bounds/Resize Messages

### sexdisplay surface ops (all client→sexdisplay, never reply)

| Opcode | Name | Payload | Direction |
|--------|------|---------|-----------|
| `0xEC` | SURFACE_CREATE_ID | sid, (y<<32)\|x, (h<<32)\|w | client→sexdisplay |
| `0xEB` | OP_SURFACE_UPDATE | sid, x, y | client→sexdisplay (position only, no w/h) |
| `0xEF` | SURFACE_FILL_RECT | sid, (sy<<32)\|sx, (sh<<16)\|sw\|color | client→sexdisplay |

**0xEB only carries x,y — not width/height.** There is no sexdisplay op to
query surface geometry, and sexdisplay never sends unsolicited messages.

### shell→app messages

| Opcode | Payload | Direction |
|--------|---------|-----------|
| `OP_QUIL_PING` (0xF0) | empty | shell→quil |
| `OP_HID_EVENT` (0x202) | scancode, value, class | shell→quil |

**No existing message carries w/h bounds to apps.**

### Conclusion

**No existing PDX message or ABI path informs Quil (or any app) of its surface
bounds at runtime.** Quil's draw constants remain hardcoded and decoupled.

---

## 3. Contract Options

### Option A: Reuse 0xEC metadata (recommended — no new ABI)

**Mechanism:** The shell already sends 0xEC with full geometry to sexdisplay
every time it tiles or re-tiles. The shell **also** sends a copy of the same
bounds data to Quil via an **existing** shell→Quil message — but no such
message currently carries w/h.

This would require a **new opcode or extending an existing one**, which is
technically an ABI addition. However, we can minimize surface area by reusing
the existing `OP_QUIL_PING` slot: make `OP_QUIL_PING` carry an optional
geometry payload.

But `OP_QUIL_PING` is already a fire-and-forget proof marker with no
expectation of a geometry payload. Overloading it violates the principle of
least surprise.

**Verdict:** Requires ABI addition → STOP FIRST per scope rules. Blocked
unless a future formal contract phase opens ABI changes.

### Option B: New shell→Quil bounds message (new ABI — blocked)

**Mechanism:** Define a new opcode e.g. `OP_SURFACE_BOUNDS` (0xFC) that the
shell sends to Quil on every tile/re-tile:

```
pdx_call(SLOT_QUIL, OP_SURFACE_BOUNDS, sid, (h<<32)|w, flags);
```

Quil stores the latest values and uses them for all subsequent 0xEF calls.

**Pros:** Clean, explicit, each layer owns its domain.
**Cons:** Adds opcode to shell→Quil ABI. Requires kernel cap grant
verification. Still no mechanism for Quil to *request* bounds on demand
(hardly needed — shell pushes on tile).

**Verdict:** Blocked until ABI changes are permitted.

### Option C: Quil keeps conservative constants until a bounds contract exists (safe — recommended now)

**Mechanism:** Quil continues to use conservative hardcoded bounds for drawing.
Sexdisplay's surface-local clamping remains the safety net for OOB fill
rects. The shell's 0xEC at boot sets the authoritative geometry on
sexdisplay; Quil doesn't need to know it to draw correctly because
sexdisplay clamps.

**Key invariant:** Sexdisplay clamps all fill rects to surface bounds
(see `0xEF` handler at sexdisplay line ~1086: sw/sh are masked to 16-bit
and drawn within surface-local coordinates). Quil can safely send oversized
coordinates and sexdisplay will clip.

**Pros:** Zero ABI changes. Zero code changes. Already works (current
behavior). No new failure modes.
**Cons:** Quil may draw outside its visible area (wasted fill rects that get
clamped). Quil cannot adapt layout to actual panel size without bounds
knowledge.

**Verdict:** Best option for current phase. Document the geometry gap
explicitly.

---

## 4. Ownership Model (Definitive)

| Domain | Owns | Does Not Own |
|--------|------|--------------|
| **silk-shell** (authority) | Layout, focus, lifecycle, z-order, surface geometry on sexdisplay, tiling policy | App-local UI drawing, fill rect content, text rendering |
| **Quil** (app server) | Panel-local UI drawing, fill rect coordinates, editor buffer state, key handling | Framebuffer, layout policy, surface geometry on sexdisplay |
| **sexdisplay** (compositor) | Framebuffer, surface registry, z-order rendering, cursor, clamping | Layout policy, lifecycle, focus, app-local draw decisions |
| **sexinput** (input) | HID event dispatch, scancode decode | Rendering, layout, focus |

**Invariant:** Only the shell decides *where* a surface is (geometry on
sexdisplay). Only Quil decides *what* it draws inside that surface.
Sexdisplay enforces the boundary via clamping.

---

## 5. Invariants

### Geometry invariants (current, Option C)

1. **Shell sets surface geometry** on sexdisplay via 0xEC. Sexdisplay stores
   it. No other domain writes surface geometry.

2. **Sexdisplay clamps all draw ops** to surface bounds. A fill rect at
   surface-local (10000, 10000) is silently clipped to the surface's actual
   w/h. This is the safety net.

3. **Quil never needs to know its surface bounds** to draw safely. It can
   send oversized 0xEF coordinates and sexdisplay clamps. Wasted fill rect
   pixels are clipped, not written to other surfaces or OOB.

4. **Shell never reads Quil's draw state.** Quil's 0xEF calls go directly
   to sexdisplay. The shell does not intercept or validate fill rects.

5. **No framebuffer pointer sharing.** Quil has SLOT_DISPLAY capability for
   0xEF/0xEC but never receives or dereferences the framebuffer address.
   sexdisplay is sole framebuffer writer.

6. **No backing buffers.** All drawing is immediate-mode 0xEF fill rects.
   No double-buffering, no swapchain, no shared-memory pixel transfer.

### Future geometry invariants (when Option B is unblocked)

7. **Shell pushes bounds to Quil** on every tile/re-tile via new opcode.
   Quil stores the latest bounds and uses them for layout-aware drawing.

8. **Both shell and Quil clamp.** Shell clamps surface geometry to screen
   bounds before sending 0xEC. Quil clamps fill rects to received bounds
   before sending 0xEF. Sexdisplay continues to clamp as final safety net.

9. **Bounds message is fire-and-forget.** Quil never replies. Shell does
   not wait for acknowledgment. Stale bounds are harmless (Quil still draws
   within the last known bounds, which are ≤ actual bounds).

---

## 6. Geometry Gap (Intentional)

There are two separate gaps to track:

### Gap A: Shell-assigned surface bounds vs full content area

The shell assigns Quil a rectangular surface on sexdisplay via 0xEC. At boot
the shell uses legacy constants (e.g. 640×480 at offset 100,100), but after
`SHELL_BOOT_TILED_DEMO_LAYOUT_V1` the shell may assign tiled geometry.
Whichever geometry the shell chooses, it is **independent** of Quil's draw
constants.

The framebuffer is typically 1280×800. The top 50px is the SilkBar strip.
This leaves ~1280×750 of content area below the bar.

| Region | x | y | w | h |
|--------|---|---|---|---|
| SilkBar strip | 0 | 0 | 1280 | 50 |
| Full content area | 0 | 50 | 1280 | 750 |

**If the shell still assigns legacy boot defaults** (640×480 at 100,100),
then the surface occupies ~51% of content width and ~64% of content height.
This is intentional — expansion to full content requires explicit
tile-on-focus or a future boot layout policy.

**If the shell assigns tiled geometry** (after
`SHELL_BOOT_TILED_DEMO_LAYOUT_V1`), this gap may be partially or fully
closed. The authoritative source is whatever 0xEC the shell sends.

### Gap B: Quil draw constants vs shell surface bounds

Quil's internal draw constants are hardcoded oversized values (e.g.
2000×2000 fill rects). These are **completely decoupled** from the shell's
surface bounds. Sexdisplay clamps all 0xEF fill rects to the actual surface
bounds, so Quil's oversized draws are harmless but wasteful.

**Closing Gap B** requires the future bounds contract (Option B in §3)
so Quil can adapt its layout to the actual panel size. This remains
blocked on ABI changes.

### Summary

| Gap | What it describes | Who owns it | Requires ABI? |
|-----|-------------------|-------------|---------------|
| A | Shell surface vs available screen | Shell layout policy | No — shell-only decision |
| B | Quil draw constants vs actual surface | Quil internal constants | **Yes** — new shell→Quil opcode |

---

## 7. Recommendation

### Phase 1 (this document — no code changes)

**Recommendation: Option C — keep conservative constants.**

No ABI changes. No code changes. Document the geometry gap. The current
behavior works because sexdisplay clamps. Quil's oversized fill rects are
harmless. The shell correctly creates and tiles the Quil surface.

### Phase 2 (future implementation — unblocked when ABI changes are OK)

**Recommendation: Option B — new OP_SURFACE_BOUNDS message.**

Smallest safe implementation prompt:

```
1. In crates/sex-pdx/src/lib.rs, define:
   pub const OP_SURFACE_BOUNDS: u64 = 0xFC;

2. In servers/silk-shell/src/main.rs, after each 0xEC upsert for
   SURFACE_ID_QUIL, call:
   pdx_call(SLOT_QUIL, OP_SURFACE_BOUNDS, SURFACE_ID_QUIL,
       (rh as u64) << 32 | rw as u64, flags);

3. In servers/quil/src/main.rs, add match arm for OP_SURFACE_BOUNDS:
   - Store w, h in static mut QUIL_SURFACE_W, QUIL_SURFACE_H
   - Use stored bounds for all subsequent 0xEF fill rect calls
   - Clamp all local draw coordinates to stored bounds before sending

4. Clamp both shell-side (before 0xEC) and quil-side (before 0xEF).
   sexdisplay retains final clamp as safety net.
```

### Phase 3 (optimization)

Once Quil knows its bounds, it can:
- Center text/UI elements within the panel
- Adjust layout when tiled to different sizes
- Stop sending oversized fill rects
- Support responsive placeholder UI (more rows in buffer list when taller)

---

## 8. Summary

| Question | Answer |
|----------|--------|
| Existing bounds route? | **Not found.** No PDX message carries surface bounds to apps. |
| ABI impact? | **Blocked until ABI changes permitted.** Option B requires new opcode. |
| Current safe option? | **Option C** — Quil uses conservative draw constants; sexdisplay clamps. |
| Ownership model? | Shell owns surface bounds on sexdisplay; Quil owns draw constants; sexdisplay enforces boundary. |
| Gap A (shell surface vs screen)? | Depends on shell layout policy. Boot defaults are 640×480; tiled layout may differ. See §6. |
| Gap B (Quil draw vs surface)? | Always present until Option B unblocked. Harmless due to sexdisplay clamp. See §6. |
| Next implementation? | **None in this phase.** ABI frozen. Mark QUIL_RUNTIME_GEOMETRY_CONTRACT as "design complete, blocked on ABI phase." |
