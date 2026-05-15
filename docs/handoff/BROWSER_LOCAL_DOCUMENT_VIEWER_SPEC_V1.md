# BROWSER_LOCAL_DOCUMENT_VIEWER_SPEC_V1

**Status:** PASS — SPEC DOCUMENT ONLY. No implementation, no code changes.
**Date:** 2026-05-16
**Depends on:** `BROWSER_PLACEHOLDER_SURFACE_V1.md` (Phase 0), `SCENE_KEYBOARD_SWITCH_PROOF_V1.md` (91-gate baseline).
**Next:** Implementation phases 1A–1E (see phase ladder).

---

## 0. PASS/FAIL

**PASS** — DOCS-ONLY SPEC. 0 gates executed, 0 faults. This is a future
design spec for Browser/WebStub Phase 1. No code was changed, no protocol
was changed, no network capability was added.

---

## 1. Spec Summary

### Target

Browser/WebStub Phase 1 delivers a **local text/document viewer** that
renders bounded, local, pre-vetted text content within the existing
placeholder surface framework. It never touches the network, never parses
HTML, and never executes JavaScript.

### Core Principle

| Rule | Meaning |
|------|---------|
| **Local only** | All content originates from local sources (static embedded text, Linen metadata, future SexFiles readback). |
| **Text only** | No HTML parsing, no CSS, no JS, no images, no fonts beyond existing system font. |
| **Bounded** | All content has known maximum size. No streaming, no infinite scroll, no unbounded allocations. |
| **Honest** | Every capability that does NOT exist is explicitly marked in markers (network=0, engine=0, fetched=0). |
| **No policy ownership** | Browser never owns surface/lifecycle policy, renderer policy, or network capability grants. Shell is the single authority. |

---

## 2. Ownership Table

| Component | Role in Phase 1 | Capabilities |
|-----------|----------------|--------------|
| **WebStub/Browser** | Owns local document viewer state: what text is loaded, scroll position, selection. Renders through sexdisplay surface (future). | Requires SLOT_DISPLAY (future) for surface, no network cap. |
| **silk-shell** | Owns placeholder/window/session policy. Creates/reaps the Browser surface. Controls focus, minimize, zoom. Owns all frame chrome. | SLOT_SHELL, SLOT_DISPLAY (existing). |
| **Linen** | Owns object/project/document metadata and naming. Browser queries Linen for document list/status (read-only, fire-and-forget). | SLOT_LINEN (future grant to browser). |
| **sexfiles** | Provides object status only (`object-status` command) until readback exists. Browser queries for "does this file exist?" metadata. | SLOT_SEXFILES (future grant). |
| **sexdisplay** | Renders browser surface pixels. Browser sends surface updates via 0xEC; sexdisplay fills the framebuffer. **Sole framebuffer writer.** | SLOT_DISPLAY (existing). |
| **Collar** | Not involved in Phase 1. Future Phase 3 (network contract). | None. |
| **Bell** | May receive browser-related events (document opened, document closed) as INFO-lane notifications. Not required for Phase 1. | SLOT_BELL (existing to Bell server). |

---

## 3. Allowed / Forbidden Table

### Allowed Local Document Sources (Phase 1)

| Source | Availability | Description |
|--------|-------------|-------------|
| Static embedded demo text | **Now** | Hardcoded text in browser binary. Proves the render path. Example: "Welcome to SexOS Browser. This is a local text viewer." |
| Linen object metadata/status | **Phase 1D** | Browser queries Linen for document names, types, sizes. Displays as a simple list. |
| SexFiles object status | **Phase 1D** | `object-status` command confirms file existence. No content readback yet. |
| SexFiles readback | **Phase 1E** | Only after storage/readback proof (`durable=1`, `sync_readback=1`). Requires separate STOP FIRST audit. |

### Forbidden (Phase 1 and beyond until separately audited)

| Claim | Why Forbidden |
|-------|---------------|
| Durable file read | Linen persist model: durable=0, sync_readback=0. Readback not proven. |
| Sync readback | Same reason. Storage maturity is object-status only. |
| POSIX path browsing | SexOS has no POSIX, no std::fs, no directory traversal. All access is PDX-capability-mediated. |
| HTTP URL open | Network=0. Phase 4+ only. |
| HTML render | Engine=0. No parser. Phase 5+ only. |
| CSS/layout engine | Engine=0. Phase 6+ only. |
| JS execution | Engine=0. Phase 8 (maybe never). |
| Network fetch | Network=0. Requires Collar network capability grants (Phase 3–4). |
| DNS resolution | Network=0. |
| TCP socket | Network=0. |
| TLS handshake | Network=0. Phase 7 only. |
| Image decoding | No image codec. Phase 6+ only. |
| Web font loading | Uses system font only (same as SilkBar/Quil). |
| User-installed extensions | No extension model. |
| Password/credential storage | No credential store. |
| Cookie/localStorage | No web storage model. |
| Cross-origin requests | Single-origin: local only. |

---

## 4. Phase Ladder

### Phase 1A — THIS SPEC (2026-05-16)
- Docs-only plan.
- No source edits, no protocol changes, no network capability.
- Handoff: `docs/handoff/BROWSER_LOCAL_DOCUMENT_VIEWER_SPEC_V1.md`

### Phase 1B — Placeholder Surface Shell-Local
- Create an actual WebStub surface (sid=202) through existing 0xEC upsert.
- Surface is a simple rectangle (e.g., 400×300, tiled alongside existing frames).
- focusable=0 (no focus stealing from Quil/Linen/Spindle).
- Browser PD does NOT write the framebuffer. Shell sends surface geometry to sexdisplay.
- Marker: `[browser.localdoc.surface] sid=202 w=400 h=300 focusable=0 ok=1`

### Phase 1C — Static Local Text Render
- Browser PD sends a fixed text string ("Welcome to SexOS Browser...") to sexdisplay via existing fill-rect protocol.
- Text appears as fill-rect rows in the browser surface.
- Uses system font only (same as Quil/SilkBar).
- No scrolling, no selection, no cursor.
- Marker: `[browser.localdoc.render] text_len=N chars=N ok=1 reason=static_embedded_demo`

### Phase 1D — Linen Object Status Panel
- Browser queries Linen for document list via OP_LINEN_OBJECTS or equivalent.
- Receives metadata (name, type, size) as a structured reply.
- Displays a simple list: document names with type badges.
- User cannot open documents yet (no readback).
- Marker: `[browser.localdoc.source] source=linen objects=N ok=1 reason=metadata_only`

### Phase 1E — Actual Local Readback
- **Requires prior storage/readback proof** (durable=1, sync_readback=1).
- Browser sends read request to sexfiles for a specific document.
- Receives bounded text content.
- Renders content in the browser surface.
- **STOP FIRST**: requires storage maturity proof. Cannot be done before `PERSISTENT_STORAGE_MATURITY_PLAN_V1` reaches readback phase.
- Marker: `[browser.localdoc.readback] source=sexfiles path=LINEN_object_N len=M ok=1 reason=readback_proven`

---

## 5. Future Markers

| Marker | Phase | Meaning |
|--------|-------|---------|
| `[browser.localdoc.spec]` | 1A | This spec document exists and has been reviewed |
| `[browser.localdoc.surface]` | 1B | Placeholder surface created (sid=202, focusable=0) |
| `[browser.localdoc.render]` | 1C | Static embedded text rendered on browser surface |
| `[browser.localdoc.source]` | 1D | Local document source queried (Linen metadata or SexFiles status) |
| `[browser.localdoc.readback]` | 1E | Actual content readback proven (requires storage maturity) |
| `[browser.localdoc.truth]` | each | Phase gate truth invariant: network=0 engine=0 fetched=0 |
| `[browser.localdoc.proof.done]` | each | Phase gate complete |

---

## 6. STOP FIRST Boundaries

| # | Boundary | Trigger Condition | Why Blocked |
|---|----------|-------------------|-------------|
| B1 | New surface protocol | Changing or extending the 0xEC surface upsert wire format | All servers that use surfaces rebuild. Shell is the surface protocol owner. |
| B2 | New storage/readback protocol | Browser sending readback PDX calls before storage maturity proof | Linen persist: durable=0, sync_readback=0. Must reach readback phase first. |
| B3 | Networking (any) | TCP socket, DNS, HTTP, TLS, or any fetch | Network=0. Requires Collar grants (Phase 3–4). |
| B4 | HTML/CSS/JS | Any HTML parser, CSS layout, or JS engine | Engine=0. Phase 5+ only. |
| B5 | Kernel/sex-pdx/global ABI edits | New syscalls, capability slots, PDX opcodes | All servers rebuild. Capability table shifts cascade. |
| B6 | Heap/std/libc/thread dependency | Using alloc, String, Vec, HashMap, or std types | SexOS constraint: no_std, no alloc, no libc, no threads. All state is static arrays. |
| B7 | Browser owning surface/focus policy | Browser calling focus_surface(), minimize_frame(), or close_surface_from_frame_light() | Shell is the single policy authority. Browser is a surface consumer only. |
| B8 | Browser writing framebuffer directly | Any direct FB pointer access | sexdisplay is the sole framebuffer writer. All rendering goes through surface fill-rect IPC. |

---

## 7. Handoff Path

```
docs/handoff/BROWSER_LOCAL_DOCUMENT_VIEWER_SPEC_V1.md
```

---

## 8. Commit Command

```bash
git add docs/handoff/BROWSER_LOCAL_DOCUMENT_VIEWER_SPEC_V1.md
git commit -m "docs(browser): local document viewer Phase 1 spec"
```

---

## 9. References

- `docs/handoff/BROWSER_PLACEHOLDER_SURFACE_V1.md` — Phase 0: WebStub placeholder (85 gates)
- `docs/handoff/BROWSER_PATH_STUBS_PACK_V1.md` — Browser path roadmap (9 phases)
- `docs/handoff/LINEN_DOCUMENT_LIFECYCLE_PLAN_V1.md` — Linen document model
- `docs/handoff/PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` — Storage maturity phases (readback blocked)
- `docs/handoff/APP_LAUNCH_EXEC_REVISIT_SLOTSHELL_V1.md` — SLOT_SHELL launch (84 gates)
- `docs/handoff/SCENE_KEYBOARD_SWITCH_PROOF_V1.md` — 91-gate baseline

---

*End of BROWSER_LOCAL_DOCUMENT_VIEWER_SPEC_V1.md*
