# QUIL_SERVER_STUB_PD_V1

**Status:** Active  
**Purpose:** Minimal no_std PD stub proving Quil exists as an isolated server binary.  
**Scope:** `servers/quil/` only + workspace Cargo.toml. No kernel/init/sexdisplay/silk-shell edits.  
**Prerequisites:** QUIL_SERVER_BOUNDARY_PLAN_V1 (6f7d150)

---

## What Was Done

Created `servers/quil/` as a standalone no_std PD stub matching the existing Linen server pattern. The stub boots, emits proof markers, and yields in a tight loop — no surface interaction, no framebuffer access, no IPC.

### Files

| File | Lines | Purpose |
|------|-------|---------|
| `servers/quil/Cargo.toml` | 8 | Package manifest, depends on sex-pdx |
| `servers/quil/src/main.rs` | 25 | no_std PD stub with DummyAllocator, boot markers, yield loop |
| `Cargo.toml` | +1 line | Added "servers/quil" to workspace members |

### Key Design Decisions

1. **NO surface creation** — The shell owns surface 201 via the existing placeholder (0xEC/0xEF). Quil stub does not call 0xEC or 0xEF. Surface ownership handoff deferred to QUIL_SURFACE_HELLO_V1.

2. **NO boot spawn** — Quil is NOT added to `kernel/src/init.rs` module_paths. It builds as a workspace member but won't appear in the ISO or be spawned at boot. This is intentional: the stub proves the PD exists and compiles, without risking boot failures or resource consumption.

3. **NO sexos_build_spec.toml change** — Quil is not packaged into the ISO until the server does something useful. Unnecessary module space consumption avoided.

4. **DummyAllocator** — Matching Linen pattern exactly. Required by sex-pdx dependency's alloc usage. Allocs return null (no heap).

5. **panic_handler** — Infinite loop, matching Linen pattern.

### Build

```
cargo build -p quil --target x86_64-sex.json -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem
```

Result: 0 errors, 0 warnings (quil only).

Note: The `-Z build-std-features=compiler-builtins-mem` flag is required for `memcmp` (emitted by core slice comparison used in sex-pdx dependencies). All server builds use this flag via the entrypoint build script.

---

## Proof Markers

| Marker | Line | When |
|--------|------|------|
| `[quil.boot]` | `main.rs:20` | Once at startup |
| `[quil.no_fb_write]` | `main.rs:21` | Once at startup, confirms no framebuffer access |

---

## STOP FIRST Items Checked

| # | Check | Result |
|---|-------|--------|
| 1 | New sexdisplay opcode? | Not needed — stub has no surface interaction |
| 2 | Kernel/init spawn changes? | STOP FIRST — adding to `module_paths` requires domain_id allocation |
| 3 | sex-pdx ABI change? | Not needed — stub only uses `sys_yield` and `serial_println` |
| 4 | Raw framebuffer access? | Not used — confirmed by `[quil.no_fb_write]` |
| 5 | Shared memory / backing buffer? | Not used |
| 6 | Cross-PD pointer? | Not used |
| 7 | App-owned frame lifecycle? | Not changed — shell still owns Quil frame |
| 8 | Filesystem authority? | Not used |
| 9 | Surface creation outside protocol? | No surface creation at all |
| 10 | Shell surface policy modification? | Not changed |

---

## Phase Sequence Progress

| Phase | Status |
|-------|--------|
| **A. QUIL_SERVER_STUB_PD_V1** | ✅ Complete |
| B. QUIL_SURFACE_HELLO_V1 | 🔲 Next |
| C. QUIL_KEY_EVENT_STUB_V1 | 🔲 |
| D. QUIL_TEXT_BUFFER_FIXED_V1 | 🔲 |
| E. QUIL_OPEN_INTENT_V1 | 🔲 |
| F. QUIL_SEX_MODE_INSPECTOR_PLAN_V1 | 🔲 |

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Minimal Quil PD stub (build only, not boot-spawned) | QUIL_SERVER_STUB_PD_V1 |
