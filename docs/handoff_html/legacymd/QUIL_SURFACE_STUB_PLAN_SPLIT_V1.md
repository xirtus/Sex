# QUIL_SURFACE_STUB_PLAN_SPLIT_V1

**Status:** Plan/split only. Do not implement.
**Purpose:** Split QUIL_SURFACE_STUB_V1 into two independently-gated phases.

---

## 1. Existing Quil Infrastructure Found

### Shell-side (silk-shell) — Complete
| Component | Status | Location |
|-----------|--------|----------|
| Surface ID | `SURFACE_ID_QUIL = 201` | `main.rs:61` |
| Frame ID | `QUIL_FRAME_ID = 3` | `main.rs:3211` |
| Boot geometry | (100, 100, 640, 480) | `main.rs:3213-3216` |
| Placeholder color | `0x0018202E` (dark slate) | `main.rs:3220` |
| Frame creation | `ensure_quil_frame()` — lazy | `main.rs:3225` |
| Open + focus | `open_quil_in_active_scene()` | `main.rs:3274` |
| Focus or open | `focus_or_open_quil()` | `main.rs:3330` |
| Toggle (F9) | `toggle_quil()` — minimize if visible, else open | `main.rs:3355` |
| SurfaceAction | `ToggleQuil` | `main.rs:662` |
| Scancode | F9 = `0x43` → `ToggleQuil` | `main.rs:759` |
| Tiling dispatch | Position updated in both tile paths | `main.rs:912, 1064` |
| Fill rect on tile | Set after geometry in both tile paths | `main.rs:922, 1077` |
| Position tracking | `SURFACE_201_X/Y/W/H` | `main.rs:1555-1558` |
| Surface alive | `true` (never destroys) | `main.rs:1676` |
| Z-order | First in z_order array | `main.rs:1751` |
| Lifecycle registered | `LifecycleState::Visible` | `main.rs:1982` |
| Frame-owned guard | Excluded when no frame | `main.rs:2029` |
| APP_SURFACES registry | `closeable: false, focusable: true` | `main.rs:118-128` |

### Server-side — Stub only
| Component | Status |
|-----------|--------|
| `servers/quil/src/main.rs` | Bare `_start()` with boot message + `sys_yield()` loop |
| `servers/quil/Cargo.toml` | Depends on sex-pdx only |
| PDX message loop | **Missing** — no `pdx_listen_raw()` call |
| Surface creation | **Missing** — no `0xEC` call |
| Fill rect | **Missing** — no `0xEF` call |
| Proof markers | **Missing** — only one `[quil.boot]` serial print |

### Kernel-side — Missing
| Component | Status |
|-----------|--------|
| Boot spawn | **Missing** — not in `module_paths` |
| SLOT_DISPLAY cap grant | **Missing** — no grant for Quil PD |
| Domain ID | Would be domain 9 (next after sexstore) |

---

## 2. Why Kernel Spawn / Cap Grant is STOP FIRST

Three distinct gates:

| Gate | Reason |
|------|--------|
| `kernel/src/init.rs` edit | Boot topology change. Adding a PD changes deterministic boot order, domain IDs, and Spawn Order contract. Must be reviewed against module_paths capacity, spawn failure handling, and boot log validation. |
| SLOT_DISPLAY grant | Authority/capability change. Every PD that can call sexdisplay directly is a new authority domain. Must verify Quil has no capability escalation path and does not bypass shell policy. |
| New spawned PD | Runtime topology change. New domain consumes scheduler resources, message ring slots, and capability table entries. Must verify Quil's resource budget fits within the kernel's static allocations. |

**All three must be approved together in a dedicated boot/cap phase.**

---

## 3. Phase 1A Scope: QUIL_SURFACE_STUB_V1A

### Allowed
- Audit existing shell-side Quil lifecycle path (silk-shell only)
- Add proof markers if gaps found (≲5 lines)
- Create handoff doc
- **No kernel edits**
- **No cap grants**
- **No sexdisplay edits**
- **No sex-pdx edits**
- **No PDX protocol changes**
- **No changes to sexos_build_spec.toml**

### Files Allowed
| File | Action |
|------|--------|
| `servers/silk-shell/src/main.rs` | Optional: ≲5 lines of proof markers only |
| `docs/handoff/QUIL_SURFACE_STUB_V1A.md` | Create: audit + proof findings |

### Files Explicitly Forbidden
| File | Reason |
|------|--------|
| `kernel/src/init.rs` | Boot topology gate |
| `servers/quil/src/main.rs` | Server spawn not yet enabled; no point fleshing out until PD boots |
| `crates/sex-pdx/src/lib.rs` | No ABI changes |
| `servers/sexdisplay/src/` | No protocol/renderer changes |
| `sexos_build_spec.toml` | Build topology part of boot gate |

### Proof Markers for 1A (if gaps found)
| Marker | Condition |
|--------|-----------|
| `[shell.quil.lifecycle.open]` | Quil opens and lifecycle state transitions correctly |
| `[shell.quil.lifecycle.minimize]` | Quil minimizes and lifecycle state follows |

### Expected Result for 1A
- Handoff doc confirming shell-side Quil lifecycle safety
- Zero or near-zero code changes
- Build passes

---

## 4. Phase 1B Scope: QUIL_PD_SPAWN_V1B

### Allowed Only After Approval
- `kernel/src/init.rs` edits
- `servers/quil/src/main.rs` fleshing out
- SLOT_DISPLAY capability grant
- PD spawn with proof markers

### Files Allowed
| File | Action |
|------|--------|
| `kernel/src/init.rs` | Add "quil" to module_paths, domain 9 handler, SLOT_DISPLAY grant |
| `servers/quil/src/main.rs` | Flesh out: 0xEC surface creation, 0xEF fill rect, PDX message loop, proof markers |
| `docs/handoff/QUIL_PD_SPAWN_V1B.md` | Create: boot/cap changes + verification |

### 1B Proof Markers
| Marker | Location | Condition |
|--------|----------|-----------|
| `✓ Spawned PD N: /quil (Domain 9)` | Kernel boot log | Quil PD created |
| `[quil.cap.grant]` | Kernel init | SLOT_DISPLAY granted |
| `[quil.boot]` | Quil `_start()` | Server boots |
| `[quil.surface.create]` | After `0xEC` | Surface created |
| `[quil.fill]` | After `0xEF` | Fill rect set |
| `[quil.loop]` | Each message loop iteration | Server listening |

### 1B STOP FIRST Conditions
| Condition | Action |
|-----------|--------|
| Framebuffer write outside sexdisplay | Halt — sexdisplay is sole FB writer |
| Editor logic | Halt — no editor until future phase |
| Linen dependency | Halt — Quil must be independent |
| sex-pdx ABI change | Halt — not approved |
| sexdisplay protocol change | Halt — not approved |

---

## 5. Recommended Order

```
Phase 1A (this turn): Audit + proof docs only
  → User approves 1A handoff
Phase 1B (next turn): Kernel spawn + server fleshing
  → User approves boot/cap topology
Future: Editor logic (not yet planned)
```

## 6. Pipeline Diagram

```
QUIL_SURFACE_STUB_PLAN_SPLIT_V1
├── Phase 1A: SHELL-ONLY PROOF
│   ├── silk-shell audit (existing infrastructure is lifecycle-safe ✅)
│   ├── optional proof markers (if gaps)
│   └── handoff doc
├── Phase 1B: PD SPAWN + CAP GRANT
│   ├── kernel/src/init.rs (3 insertions)
│   ├── servers/quil/src/main.rs (flesh out server)
│   ├── SLOT_DISPLAY grant
│   └── handoff doc
└── Future: Quil editor logic (not yet planned)
```
