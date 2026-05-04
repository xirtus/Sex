# QUIL_SURFACE_HELLO_PLAN_V1

**Status:** Active  
**Purpose:** Plan the safest path for Quil PD to draw/request a hello placeholder for surface 201 using existing protocols.  
**Scope:** Docs only. No code changes.  
**Prerequisites:** QUIL_SERVER_STUB_PD_V1 (6c1f3b5)

---

## 1. Protocol Audit

### Surface ownership (sexdisplay)

Every surface in sexdisplay has an `owner_pd` field, set on first 0xEC create and verified on every subsequent mutation:

| Opcode | Ownership check | Effect |
|--------|----------------|--------|
| **0xEC** (create/update) | `owner_pd != caller_pd` → reject | Only owner can reposition |
| **0xEF** (fill rect) | `owner_pd != caller_pd` → reject | Only owner can set fill |
| **0xEE** (destroy) | `owner_pd != caller_pd` → reject | Only owner can destroy |

### Current surface 201 ownership

- **Created by:** `silk-shell` via `pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_QUIL, ...)` in `tile_visible_frames()` (line 791)
- **Owner PD:** silk-shell's PD (set by sexdisplay at creation time)
- **Quil PD status:** Not spawned, no surface access

### PD routing

- `servers/quil` is **not boot-spawned**, **not ISO-packaged**
- `servers/quil` has no PDX message route to/from any other PD
- `silk-shell` uses `pdx_listen_raw(0)` for incoming messages — no existing shell→server event forwarding
- No `SLOT_QUIL` constant exists in sex-pdx

### Key constraint

> Quil PD cannot call 0xEF on surface 201 because **owner_pd mismatch** → sexdisplay rejects with `AUTH: 0xEF fill rejected sid=201 caller=<quil> owner=<shell>`.

---

## 2. Options

### Option A: Shell-mediated hello

Quil sends an intent/event to silk-shell; the shell calls 0xEF on its behalf.

| Dimension | Assessment |
|-----------|-----------|
| Required files | `crates/sex-pdx/src/lib.rs` (new slot?), `servers/silk-shell/src/main.rs` (new message handler), `servers/quil/src/main.rs` (send logic) |
| Required protocol | New PDX message type or opcode for shell←quil communication — **does not exist** |
| Risks | Drift toward shell-as-broker pattern; new opcode = ABI change; SLOT allocation for Quil |
| STOP FIRST | ⛔ **New sex-pdx slot or opcode** — violates ABI freeze |
| Smallest implementation | Add `SLOT_QUIL` constant, shell listens for quil events, quil sends "draw" intent |
| Proof markers | `[quil.draw.request]`, `[shell.quil.mediation]` |

**Verdict: STOP FIRST. Rejected for now.**

### Option B: Display-mediated existing fill

Quil calls existing sexdisplay fill/update protocol for surface 201.

| Sub-option | Approach | Blocked by |
|------------|----------|-----------|
| **B1** | Quil claims ownership (shell 0xEE, Quil 0xEC) | User directive: no lifecycle ownership change yet |
| **B2** | sexdisplay allows any PD to 0xEF any surface | sexdisplay change = STOP FIRST |
| **B3** | shell delegates ownership token to Quil | New protocol = STOP FIRST |

**Verdict: All B sub-options blocked. Rejected for now.**

### Option C: Stay shell-only (recommended now)

Quil remains a build-only PD stub. The shell keeps the existing placeholder (0xEF fill rect via `tile_visible_frames()`). No runtime surface interaction attempted.

| Dimension | Assessment |
|-----------|-----------|
| Required files | None — status quo |
| Required protocol | None — existing shell→display path unchanged |
| Risks | None — already deployed and working |
| STOP FIRST | None |
| Implementation | Already done (QUIL_VISUAL_PLACEHOLDER_V1) |
| Proof markers | `[quil.boot]`, `[quil.no_fb_write]` (stub), `[shell.tile]`, `[shell.quil.open]` (shell placeholder) |

**Verdict: Safe. Recommended for now.**

---

## 3. Option Comparison

| Criteria | A (shell-mediated) | B (display-mediated) | C (stay shell-only) |
|----------|-------------------|---------------------|---------------------|
| Code changes | High | Medium | None |
| New opcode required | Yes | No* | No |
| sexdisplay change | No | Yes (B2) | No |
| Lifecycle risk | Low | High (B1) | None |
| Blocked | Yes (STOP FIRST) | Yes (STOP FIRST) | No |
| Ships today | No | No | ✅ |

*\*B1 uses existing opcodes but changes surface ownership — prohibited.*

---

## 4. Recommended Path

```
C now → QUIL_BOOT_MODULE_PLAN_V1 → QUIL_BOOT_SPAWN_V1 → revisit A or B
```

### Why not A or B now

- Quil isn't spawned. Runtime hello requires a running PD.
- Spawning requires boot module packaging + kernel init entry or shell-initiated spawn.
- Even after spawn, drawing on surface 201 requires either:
  - **Ownership transfer** (shell releases, Quil claims) — lifecycle risk, user vetoed
  - **New protocol** (shared surface or mediated draw) — ABI change, STOP FIRST
  - **sexdisplay relaxation** (allow non-owner fill) — sexdisplay change, STOP FIRST

All three runtime paths are blocked without prerequisite decisions about boot packaging, surface ownership policy, or ABI evolution.

### What to unblock first

**QUIL_BOOT_MODULE_PLAN_V1** — decide if/when `servers/quil` becomes:
1. Packaged in ISO (added to `sexos_build_spec.toml`)
2. Loaded as a Limine module at boot
3. Spawned by kernel init (added to `module_paths` with domain_id=9)
4. Or spawned dynamically by shell at runtime (no kernel change)

---

## 5. STOP FIRST List

| # | Condition | Relevant to |
|---|-----------|-------------|
| 1 | New sex-pdx slot constant (`SLOT_QUIL`) | Option A |
| 2 | New PDX opcode for shell←quil messaging | Option A |
| 3 | sexdisplay owner_pd check relaxation | Option B2 |
| 4 | Shell-initiated surface ownership transfer | Option B1 |
| 5 | Kernel init spawn without boot module plan | Any spawn attempt |
| 6 | sexos_build_spec.toml edit without boot plan | ISO packaging |

---

## 6. Next Phase

→ **QUIL_BOOT_MODULE_PLAN_V1** (docs-only)
   - Decide if/when Quil becomes ISO-packaged and boot-spawned
   - Evaluate kernel init domain_id allocation (currently 1-8 used)
   - Evaluate vs. shell-initiated dynamic spawn (no kernel change)
   - Do NOT implement spawn yet

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Surface hello path analysis — recommended stay shell-only for now | QUIL_SURFACE_HELLO_PLAN_V1 |
