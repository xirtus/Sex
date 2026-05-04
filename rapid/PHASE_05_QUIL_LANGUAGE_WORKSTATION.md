# PHASE 05: Quil — The Language Workstation

## Revolutionary Vision

Quil is not an IDE.
Quil is not Cursor.
Quil is not TextMate.
Quil is not OnlyOffice.

**Quil is a language workstation.**

A workspace for:
- writing text
- writing code
- designing interfaces
- editing docs
- reviewing diffs
- building SexOS
- prompting agents
- managing language servers
- inspecting ASTs
- running proof/build loops
- visualizing PDX/capability impact of code

Quil is not a Linux IDE ported to SexOS. Quil is SexOS-native — it understands the operating system it runs on, the laws that govern it, and the graph of capabilities, PDs, and surfaces that make it work.

## Ownership
- **Quil** (app/exclusive): all workstation modes, project tree, console, inspector panels, Sex Mode awareness
- **Linen** (data source): project tree, file open/save, object graph
- **Mesh** (read-only): system graph data for inspector — live PDX routes, capability edges, device topology
- **Collar** (read-only): authority data for inspector — grant state, revocation history, capability provenance

## The Modes

Quil is mode-based. Each mode transforms the workspace for a different kind of creation:

### Text Mode
Prose, markdown, notes, docs, manuscripts. Simple editing with keyboard-driven workflow. No glyph pipeline needed in V1 — colored blocks communicate structure (headers, paragraphs, lists) through position and color.

### Code Mode
Rust, Zig, C, assembly, config, scripts. Syntax-aware editing with token matching, bracket matching, and SexOS-aware diagnostics. Flat ARGB colored blocks per token category (keyword = blue, string = green, comment = gray, type = yellow).

### Sex Mode — THE UNIQUE MODE
SexOS-native development awareness. This is what no other IDE can do:

```
You edit servers/silk-shell/src/main.rs.

Quil Sex Mode shows beside the code:
- Producer: silk-shell
- Consumer: sexdisplay
- IPC: OP_SILKBAR_UPDATE
- Forbidden: framebuffer write
- Required: model contract validation
- Build gate: ./scripts/entrypoint_build.sh
- Risk: ABI drift if UpdateKind discriminant changes
```

**Quil SexOS Superpowers:**
1. PDX call graph beside code — "this function calls sexdisplay, here are the opcodes"
2. Capability slot map beside code — "this server holds slots 3, 4, 7"
3. no_std violation scanner — "this import requires std, which is forbidden"
4. unsafe/PKRU/MPK risk scanner — "this unsafe block touches kernel memory"
5. Kernel/ABI edit STOP-FIRST detector — "this file changes the kernel ABI, halt and plan"
6. Framebuffer ownership detector — "this code writes to the framebuffer without authorization"
7. Mesh/Collar integration — "this capability change affects 3 downstream PDs"

### Design Mode
UI layout, Silk scenes, visual language, app mockups. Design surfaces by placing colored blocks in a grid that represents the framebuffer. No pixel editing — just structural layout that translates to Silk scene tokens.

### Review Mode
Diffs, audits, invariants, build errors, proof markers. Side-by-side diff view where each changed line is a colored block. Build markers are clickable — red = panic, green = pass, yellow = warning.

### Agent Mode
Claude/Codex/Gemini/Qwen prompt orchestration. Structured prompt builder with:
- Project context (what files exist, what phase is active)
- Code selection (send selected blocks as context)
- Build/proof results (include latest build output)
- Handoff updater (generate handoff docs from agent sessions)

### Office Mode (future)
Documents, tables, diagrams, presentations. Deferred until text glyph pipeline exists.

## What Already Exists
- No Quil server or surface exists yet
- `SURFACE_ID_QUIL` not allocated
- SexOS apps use `0xEC` create → 0xFD tab info → focus pattern (well-established)
- `OP_HID_EVENT` delivers keyboard input to silk-shell → can forward to Quil
- Linen (Phase 4) will provide file data — Quil depends on it
- Synthetic keyboard input works (scancode → SurfaceAction dispatch works; app keyboard forwarding is separate)
- Mesh/Collar (Phase 6) provides the system graph Quil visualizes

## Bundle

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| Quil surface create | Register Quil as a PDX server, create surface on launch, standard chrome | 4h | HIGH — foundation |
| Keyboard routing | Forward keyboard input from silk-shell to Quil surface via OP_HID_EVENT | 3h | HIGH |
| Text mode | Keyboard input → edit buffer → render to surface (colored blocks, no glyphs) | 8h | HIGH |
| Sex Mode — PDX graph overlay | Beside code, show PDX call graph for the open file | 6h | HIGH — unique value |
| Project tree via Linen | Linen data source for file navigation inside Quil | 3h | High (after Phase 4) |
| Build/proof console | Capture build output, display proof marker results, colored pass/fail | 6h | High |
| Code mode (syntax tokens) | Token matching for Rust keywords, strings, comments — colored blocks | 5h | Medium |
| Sex Inspector panel | PDX call count, capability slots, no_std warnings, framebuffer ownership | 6h | Medium |
| Sex Mode — no_std scanner | Scan open file for std imports, unsafe blocks, kernel ABI dependencies | 4h | Medium |
| Agent Mode | Structured prompt builder with project context injection | 4h | Low |
| Review Mode | Side-by-side diff with colored-block change indicators | 4h | Low |
| Design Mode | Structural layout mode for Silk scene design | 5h | Low |
| Markdown/doc mode | Render markdown structure as colored block layout | 3h | Low |
| Server ownership map | Color-coded indicators showing which PD owns each function/surface | 4h | Medium |

## Smallest First Step
Create a Quil surface that receives keyboard input and displays typed characters as colored rectangles. No text rendering, no glyphs — just prove the surface lifecycle (0xEC → focus → keyboard event → redraw) works for Quil. Then immediately layer on Sex Mode's PDX graph overlay — showing beside each open file which PDX calls it makes. This proves Quil's unique value from day one.

## Dependencies
- **Blocking**: Phase 4 (Linen) for project tree, Phase 6 (Mesh/Collar) for inspector data
- **Blocked by**: Phase 4 and Phase 6 for full feature set, but text mode and Sex Mode's PDX scanner can start immediately with hardcoded data
- **Can parallelize with**: Phase 4's UI work, Phase 6's graph model (Quil consumes Mesh API)

## Visual Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ SilkBar: Scene=Kernel Work  Build=green  PD faults=0           │
├─────────────────┬───────────────────────────┬───────────────────┤
│  Project Tree   │  Code / Text / Design     │  Sex Inspector    │
│  Linen objects  │  Canvas                    │  PDX graph        │
│  docs/          │                            │  caps/slots       │
│  servers/       │  fn send_update(...)       │  ABI warnings     │
│  kernel/        │                            │                   │
├─────────────────┴───────────────────────────┴───────────────────┤
│  Proof Console: build result, qemu markers, fault scan           │
└─────────────────────────────────────────────────────────────────┘
```

Three-panel layout:
- **Left**: Project tree (Linen objects — files, docs, projects)
- **Center**: Active mode canvas (text, code, design, review)
- **Right**: Sex Inspector (live PDX graph, capability slots, ABI status, no_std warnings)
- **Bottom**: Proof Console (build output, pass/fail markers, QEMU traces)

## The Trinity

```
TRINITY="
Quil edits the system.
Mesh shows what the system is and how it connects.
Collar controls who/what has authority.
"
```

**World-changing workflow:**

1. In Quil, coder changes sexinput HID route
2. Quil warns: "this touches input authority and shell focus path"
3. Mesh previews affected graph: `sexusb → sexinput → silk-shell → sexdisplay`
4. Collar shows required authority: "sexinput may emit OP_HID_EVENT, app surfaces may not receive raw hardware input"
5. Build/proof runs
6. If patch passes, handoff records recurring invariant

This is the SexOS development experience. No other OS has it because no other OS was built from the ground up with capability-native architecture.

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| No text glyph rendering pipeline exists | HIGH | HIGH | Colored blocks/rectangles per character. No glyphs needed for V1 MVP. Text is communicated via shape, color, position — not font. |
| Keyboard input routing to app surfaces not built | High | High | Forward via OP_HID_EVENT (0x202) with destination surface_id. silk-shell already receives scancodes. |
| Sex Mode PDX scanner needs static analysis of source | Medium | Medium | Start with simple regex/string matching for `OP_*` constants and PDX slot references. Full AST analysis deferred to V2. |
| Inspector data depends on Mesh/Collar (Phase 6) | Medium | Medium | V1 uses hardcoded mock data for the inspector panels. Real data integration happens after Phase 6 stabilizes. |
| Quil becomes too complex (7 modes is a lot) | Medium | Low | Modes are progressive. Text mode ships first. Code + Sex modes ship next. Agent/Design/Review/Office are later. Users see only the modes they use. |

## Exit Criteria (Done Checklist)
- [ ] Quil PDX server boots and creates a surface on launch
- [ ] Quil surface is focusable and receives keyboard input
- [ ] Text mode: edit buffer with cursor, insert/delete, text as colored blocks
- [ ] Sex Mode: PDX call graph overlay beside open file (hardcoded data first, then real from Mesh)
- [ ] Code mode: token-colored blocks (keywords, strings, comments, types)
- [ ] Build/proof console: shows build output with pass/fail coloring
- [ ] Sex Inspector panel: PDX call count, capability slots, no_std warnings
- [ ] Project tree via Linen integration (if Phase 4 complete)
- [ ] Three-panel layout functional (tree + canvas + inspector)
- [ ] Build passes. Boot passes. No panic.

## Testing Strategy
- **Surface lifecycle**: Boot QEMU, verify Quil surface appears and is focusable
- **Keyboard input**: Type via synthetic keyboard, verify Quil receives and displays characters
- **Sex Mode**: Open a known PDX caller (silk-shell), verify Quil displays its PDX calls
- **Build console**: Run build from Quil, verify output displayed with pass/fail coloring
- **Inspector**: Verify PDX call count, slot map, no_std warnings display
- **Regression**: All existing markers fire

## Efficiency Opportunity
**Ship Text Mode + Sex Mode first. Skip the rest until they're needed.** Text mode proves the surface lifecycle and keyboard routing. Sex Mode proves Quil's unique value — no other editor can show PDX call graphs beside code. Code mode, Review mode, Agent mode, Design mode are all secondary to proving "Quil understands SexOS."

Sex Mode's PDX scanner doesn't need real static analysis in V1. Hardcode a map of "filename → known PDX calls" for the first 5 server files, then build the real scanner iteratively as Quil is used to edit more files.

## Completeness Gain
Development tooling: **5–20% → 55–70%** (revised upward because Quil is not "a text editor" — it is a language workstation with SexOS-native superpowers no other editor can match)

## Files Changed
- `servers/quil/src/main.rs` (new server — Quil surface, mode dispatch, Sex Inspector, proof console)
- `servers/silk-shell/src/main.rs` (Quil surface lifecycle, keyboard forwarding via OP_HID_EVENT)
- `servers/sexdisplay/src/main.rs` (Quil surface rendering — colored blocks, layout panels)
- `crates/sex-pdx/src/lib.rs` (opcodes for Quil↔shell, Quil↔Mesh, Quil↔Collar communication)

## Forbidden
- Full LSP implementation (deferred — Sex Mode's PDX awareness is more important than IDE features)
- True text glyph rendering (deferred — colored blocks are sufficient for V1)
- Porting VS Code/Cursor/Lapce to SexOS (Quil is SexOS-native, not a port)
- Clone Denim (build the minimum needed to develop SexOS, then extend)
- Mode overload (ship Text + Sex modes first, add others only when needed)

## Next Phase
PHASE_06_MESH_CAPABILITY_GRAPH.md
