# QUIL_SERVER_BOUNDARY_PLAN_V1

**Status:** Active  
**Purpose:** Exact implementation boundary for creating `servers/quil` as a no_std PDX app server.  
**Scope:** Docs only. No code changes.  
**Prerequisites:** QUIL_VISUAL_PLACEHOLDER_V1 (809e41a), APP_SURFACE_REGISTRY_V1 (ca24f9c)

---

## 1. Ownership Model

| Layer | Owns | Does NOT own |
|-------|------|-------------|
| **silk-shell** | Scene/Frame/Tab/focus/lifecycle, surface policy (focusable/closeable), keyboard dispatch, tiling, chrome geometry | Surface pixel content, editor state, file I/O |
| **sexdisplay** | Framebuffer composition, surface slots, z-order, chrome rendering, cursor | Editor state, file access, keyboard routing |
| **servers/quil** (future) | Editor state, text buffer, cursor position, selection, draw request content | Framebuffer writes, surface lifecycle, frame chrome, file system |
| **servers/linen** (future) | File/object browsing, open intent dispatch | Editor state, framebuffer |

### Invariant

> silk-shell decides *when* Quil is visible (scene, tile, focus).  
> sexdisplay decides *how* Quil is rendered (pixels, chrome, layer).  
> servers/quil decides *what* content to display — but only via existing protocol (0xEF fill rect, or later 0x?? draw commands if added via separate sexdisplay protocol change).

No layer crosses its boundary.

---

## 2. V1 Server Goals

### 2.1 Boot as isolated no_std PD

- Standalone `servers/quil/src/main.rs` with `#![no_std]`, `#![no_main]`
- `extern "C" fn _start() -> !`
- Dummy allocator (same pattern as `servers/linen/src/main.rs`)
- Single PDX receive loop (`sys_yield` polling or `pdx_await`)
- No `servers/quil/Cargo.toml` workspace addition until build-tested

### 2.2 Startup marker

```rust
serial_println!("[quil.boot]");
```

### 2.3 Surface creation or acknowledgment

Quil's surface (201) currently exists as a placeholder created by the shell via 0xEC. The Quil server has two options:

**Option A — Let shell own surface (simpler for V1):**
- Quil server does NOT create surface 201
- Shell continues to own and position it via 0xEC/0xEF
- Quil server uses 0xEF to update fill rect content
- **STOP FIRST if** sexdisplay rejects 0xEF from non-owner PD

**Option B — Claim surface from shell (cleaner ownership):**
- Shell destroys placeholder (0xEE) before starting Quil server
- Quil server creates surface 201 (0xEC) at boot, becoming owner
- **STOP FIRST if** no mechanism exists for shell to release ownership

**Recommendation for V1:** Option A (shell owns surface, Quil only uses 0xEF to draw content). Sexdisplay's 0xEF handler checks `owner_pd` — so Option A requires that the shell's PD is the owner and Quil's 0xEF would be rejected. If this is the case, Option B becomes necessary.

**Decision deferred to QUIL_SURFACE_HELLO_V1** — test whether 0xEF from Quil PD is accepted for a shell-owned surface.

### 2.4 No framebuffer writes

- Quil must never write to the framebuffer
- No raw pointer dereference to FB_PTR
- All visual output goes through sexdisplay opcodes (0xEF fill, or later opcodes)
- Proof marker: `[quil.no_fb_write]`

### 2.5 No file access

- Quil V1 has no file I/O capability
- No filesystem authority
- No sexstore integration
- File open intent deferred to Linen integration phase

### 2.6 No keyboard routing

- Quil V1 does not request keyboard input
- No HID event subscription
- All keyboard dispatch remains in silk-shell
- Keyboard routing added in QUIL_KEY_EVENT_STUB_V1 if a safe existing event path exists

---

## 3. Required STOP FIRST Conditions

Any of these triggers a halt and document revision:

| # | Condition | Rationale |
|---|-----------|-----------|
| 1 | **New sexdisplay opcode** | Opcode ABI freeze — new ops require sexdisplay protocol change + handoff doc |
| 2 | **Kernel/init spawn changes** | Quil server must start within existing boot infrastructure or be spawned by shell at runtime |
| 3 | **sex-pdx ABI change** | PDX is the sole IPC mechanism — no new message types, no ABI break |
| 4 | **Raw framebuffer access** | sexdisplay is sole framebuffer writer — violation breaks MPK isolation |
| 5 | **Shared memory / backing buffer** | No cross-PD shared memory exists yet — would require kernel changes |
| 6 | **Cross-PD pointer** | No dereference of pointers from other PDs — safety invariant for MPK |
| 7 | **App-owned frame lifecycle** | silk-shell owns all frame/tile/scene lifecycle — Quil cannot self-position |
| 8 | **Filesystem authority** | No FS access until servers/sexstore or similar exists |
| 9 | **Surface creation outside existing protocol** | Must use 0xEC/0xEB/0xEE/0xEF only — no new surface control paths |
| 10 | **Shell surface policy modification** | Quil must not change its focusable/closeable/lifecycle policy in the shell |

---

## 4. Phase Sequence

### A. QUIL_SERVER_STUB_PD_V1 *(next)*
- Create `servers/quil/` with Cargo.toml, main.rs
- no_std PD with dummy allocator, boot marker
- Infinite loop with `sys_yield` — no surface interaction yet
- Add to workspace, build test
- **Proof:** `[quil.boot]`

### B. QUIL_SURFACE_HELLO_V1
- Test 0xEF fill from Quil PD on shell-owned surface 201
- If accepted: Quil updates its own placeholder fill (color change)
- If rejected: fallback to Option B (claim surface from shell)
- **Proof:** `[quil.surface.201]`, `[quil.no_fb_write]`

### C. QUIL_KEY_EVENT_STUB_V1
- Route keyboard events from silk-shell to Quil via existing PDX event path
- Quil receives and logs key events — no action yet
- **STOP FIRST if** no existing safe HID event dispatch exists
- **Proof:** `[quil.event.recv]`

### D. QUIL_TEXT_BUFFER_FIXED_V1
- Fixed-size text buffer in Quil (no allocator needed)
- Cursor movement, character insert at cursor
- Rerender via 0xEF with cursor indicator
- **Proof:** `[quil.draw.request]`

### E. QUIL_OPEN_INTENT_V1
- Linen sends "open file" intent to Quil via PDX
- Quil loads fixed-size file content into buffer
- Requires minimal file-read protocol (sexstore or direct PDX)
- **Deferred until Linen integration exists**

### F. QUIL_SEX_MODE_INSPECTOR_PLAN_V1
- Sex-mode inspection of kernel state from Quil
- Read-only debug views
- Requires kernel debug IPC — deferred

---

## 5. Proof Markers

| Marker | Phase | When |
|--------|-------|------|
| `[quil.boot]` | A | After PD init, before main loop |
| `[quil.surface.201]` | B | After successful 0xEF/0xEC for surface 201 |
| `[quil.no_fb_write]` | B | Once per boot confirming no framebuffer access |
| `[quil.event.recv]` | C | On each routed key event |
| `[quil.draw.request]` | D | On each 0xEF content update |

All markers use budgeted logging (max 4-16 prints) to prevent serial spam.

---

## 6. Non-Goals (exact handoff warnings)

- **No text editor feature** in the shell or sexdisplay
- **No language server protocol** — SexOS has no JSON parser, no async I/O
- **No multi-client editing** — single user, single instance
- **No syntax highlighting** — would require parser in no_std
- **No file browser** — that's Linen's domain
- **No mouse-driven cursor positioning** — too complex for V1, requires sexdisplay hit-test extension
- **No scrollback buffer** — fixed buffer only
- **No undo/redo** — deferred to QUIL_TEXT_BUFFER_ENHANCED_V2
- **No clipboard** — no cross-PD clipboard protocol exists

---

## 7. Build for Phase A

```bash
cargo new --name quil servers/quil
# Add to workspace Cargo.toml
# Mirror linen's Cargo.toml (dependencies: sex-pdx, sex-rt)
cargo build -p quil --target x86_64-sex.json -Z build-std=core,alloc
```

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Quil server boundary plan (docs only) | QUIL_SERVER_BOUNDARY_PLAN_V1 |
