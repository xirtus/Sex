# H2: Quil Workstation Model

**Status:** Handoff (docs only — no implementation)
**Commit:** _(to be committed)_
**Build:** N/A (docs only)

## 1. Purpose

Define Quil as a language/workstation surface within SexOS. Quil is **not just a
text editor** — it is the development environment for SexOS-native software:
text, code, design, agent interaction, build coordination, and system
introspection all through a unified workstation paradigm.

### What Quil IS (H2 scope)
- Workstation model definition
- Object types, views, operations
- Relationship to Linen/Collar/Mesh/Bell
- Behavioral invariants for future implementation

### What Quil IS NOT (H2 scope)
- Real editor implementation (deferred to H3+)
- Parser or compiler (deferred)
- Filesystem or storage owner (E track)
- Authority owner (Collar owns grants — F2)
- Renderer (sexdisplay sole framebuffer writer)
- Agent runtime (future)

### Key design constraints
- **No POSIX assumptions:** no paths, no /dev, no fd, no env, no argv
- **no_std Rust** throughout
- **Static arrays only** — no heap allocation for buffer/workspace state
- **Collar-gated authority** for all sensitive operations (save, build, launch)
- **Linen-referenced storage** — buffers reference Linen object IDs, not raw files
- **Mesh-visualizable diagnostics** — errors/warnings as Mesh graph nodes later

## 2. Workstation Object Types

| # | Object Kind | Description | Persistence |
|---|-------------|-------------|-------------|
| 1 | **Buffer** | In-memory editable text/code content | Memory-only until H5+ |
| 2 | **Document** | Named unit with metadata (language, modified flag) | Linen-backed after H5 |
| 3 | **CodeUnit** | Structural region: function, class, block, paragraph | Derived from parse (H4+) |
| 4 | **ProjectWorkspace** | Collection of documents + build target + config | Memory-only H2 |
| 5 | **BuildTarget** | Output spec: kernel module, server, app, package | References Collar grants |
| 6 | **Diagnostic** | Compiler error/warning/info with location | Mesh-visualizable later |
| 7 | **ReviewNote** | Annotated comment on a document/code region | Linen object after H5 |
| 8 | **DesignNote** | Structured design intent (not free text) | Linen object after H5 |
| 9 | **AgentTask** | Agent work item: refactor, search, generate | Placeholder in H2 |
| 10 | **LinenObjectRef** | Reference to a Linen object (document, file, note) | In-memory handle |
| 11 | **MeshDiagnosticRef** | Reference to a Mesh graph node for visualization | In-memory handle |
| 12 | **BellEventRef** | Reference to a Bell event (build fault, review request) | In-memory handle |

## 3. Object Fields

Each workstation object carries:

| Field | Type | Optional? | Description |
|-------|------|-----------|-------------|
| `quil_object_id` | u64 | No | Unique object identifier within Quil instance |
| `object_kind` | u8 | No | Enum: 0=Buffer, 1=Document, 2=CodeUnit, ..., 12=BellEventRef |
| `display_name` | `[u8; 64]` | No | Fixed-cap display name (truncated if longer) |
| `linen_object_ref` | u64 | Yes | Linen object ID if backed by stored content (0 = none) |
| `project_id` | u64 | Yes | ProjectWorkspace ID this object belongs to (0 = unaffiliated) |
| `buffer_state` | u32 | Yes | Buffer byte length or state flags (0 = not loaded) |
| `dirty_flag` | bool | No | Has uncommitted edits since last save/snapshot |
| `language_mode` | u8 | Yes | Language token: 0=Plain, 1=Rust, 2=Quil, 3=SexOS-config, ... |
| `grant_ref` | u64 | Yes | Collar grant capability ID for authority-sensitive ops (0 = none) |
| `privacy_class` | u8 | No | Redaction class: 0=Public, 1=Internal, 2=Confidential, 3=Secret |
| `diagnostic_count` | u16 | No | Number of active diagnostics for this object |
| `scene_frame_tab` | u32 | Yes | Packed `(scene<<16)|(frame<<8)|tab` if open in workspace (0 = not shown) |
| `audit_marker` | u64 | Yes | Proof marker generation for lifecycle tracking (0 = unverified) |

### Field invariants
- `object_kind` must always be a valid enum variant (no out-of-range values)
- `display_name` is zero-terminated, never contains uninitialized bytes
- `dirty_flag` is meaningful only if `buffer_state > 0`
- `privacy_class` must never be downgraded without Collar re-approval (future)

## 4. Modes / Views

Quil workstation provides mode-specific views. Each mode is a rendering
perspective, not a state machine — Quil can show multiple modes via tab splits
later.

| # | Mode | Purpose | H2 Status |
|----|------|---------|-----------|
| 1 | **Text** | Plain text editing | Placeholder |
| 2 | **Code** | Source code editing with syntax region tracking | Placeholder |
| 3 | **SexOS** | SexOS config, manifest, build spec editing | Placeholder |
| 4 | **Design** | Structured design note authoring | Placeholder |
| 5 | **Review** | Review note annotation overlay | Placeholder |
| 6 | **Agent** | Agent task prompt/result view | Placeholder (stub) |
| 7 | **ProjectOutline** | Tree view of ProjectWorkspace documents | Placeholder |
| 8 | **DiagnosticsPanel** | Error/warning list from compiler/analysis | Placeholder |
| 9 | **BuildOutput** | Build log and status | Placeholder |

### Mode constraints
- No mode owns framebuffer rendering — all modes submit content via existing
  shell display primitives (0xEC/0xEF/0xEA)
- Mode switching is Quil-local; shell only sees surface content changes
- Each mode can be Collar-gated individually if privacy-class sensitive

## 5. Operations

### Buffer operations (H3+)
| Operation | Collar-gated? | Description |
|-----------|---------------|-------------|
| `open_buffer(obj_ref)` | No | Open existing buffer or Linen object into editor view |
| `close_buffer(buf_id)` | No | Close buffer, prompt save if dirty |
| `switch_buffer(buf_id)` | No | Switch active buffer in current tab |
| `edit_request(buf_id, delta)` | No | Apply edit delta to buffer content |
| `save_request(buf_id)` | **Yes** | Persist buffer to Linen storage via Collar grant |
| `run_build(target_id)` | **Yes** | Execute build pipeline via Collar grant |

### Workspace operations (H4+)
| Operation | Collar-gated? | Description |
|-----------|---------------|-------------|
| `create_workspace(name)` | No | Create new ProjectWorkspace |
| `add_document(ws_id, doc_ref)` | No | Link document to workspace |
| `remove_document(ws_id, doc_ref)` | No | Unlink document from workspace |

### Integration operations (H5+)
| Operation | Collar-gated? | Description |
|-----------|---------------|-------------|
| `send_to_linen(obj_ref)` | No | Send object reference to Linen for storage |
| `show_diagnostic(diag_ref)` | No | Highlight diagnostic in Mesh panel |
| `request_authority(auth_kind)` | **Yes** | Request capability from Collar for op |
| `link_bell_event(event_ref)` | No | Associate Bell event with current diagnostic |

### Operation invariants
- All Collar-gated operations must check grant_ref before execution
- No operation creates POSIX file handles or paths
- No operation writes to framebuffer or system state outside Quil surface
- Open/close/switch track lifecycle through existing LIFECYCLE_TABLE

## 6. Invariants

1. **Collar non-bypass:** Quil never bypasses Collar for authority-sensitive
   operations (save, build, launch, grant access). Attempted bypass =
   `[quil.authority.reject]` proof marker + no-op.

2. **No POSIX assumptions:** Quil never constructs or assumes POSIX paths,
   file descriptors, environment variables, or argv. All storage references
   are Linen object IDs or capability refs.

3. **No raw cross-PD pointers:** Quil never stores or dereferences raw
   pointers across protection domains. All inter-server references go
   through PDX capability handles.

4. **No framebuffer writes:** Quil never writes to the framebuffer directly.
   All visual output goes through shell display primitives (0xEC geometry,
   0xEF damage, 0xEA cursor).

5. **No parser/compiler in H2:** Quil does not implement parser, compiler,
   or build pipeline in H2. Language mode tokens are metadata only.

6. **Linen-referenced storage:** Buffers reference Linen objects or
   capability refs, not raw filesystem paths or inode numbers. No direct
   storage access.

7. **Degrade safely:** Missing object references, invalid grant refs, and
   unavailable backing storage never cause panics. Quil surfaces
   "unavailable" state and continues operating on available objects.

8. **Renderer never owns policy:** sexdisplay and shell never make editor
   policy decisions (save prompts, build scheduling, authority checks).
   Quil owns all workstation policy.

## 7. Relationship to Existing Work

| Existing Work | Relationship |
|---------------|--------------|
| **E1/E2 Quil placeholder** | Placeholder lifecycle path (Surface ID 201, Frame ID 3, F9 toggle). H2 model defines what the real Quil surface will show. |
| **H1 Linen object model** | Quil buffers reference Linen object IDs for storage. H1's 11 object types, 7 views, 11 operations define the storage layer Quil reads/writes. |
| **B1-B4 Scene/Frame/Tab** | Quil uses existing Frame 3 / Tab 0 lifecycle for visibility. H2 model adds scene_frame_tab field to track open state. |
| **C1-C3 Atlas** | Quil appears in Atlas as Frame 3 card. No Atlas changes needed. |
| **F1 Mesh diagnostic model** | Diagnostics panel references Mesh node types for visualization later. CodeUnit objects can map to Mesh structural nodes. |
| **F2 Collar authority map** | All save/build/grant operations require Collar capability. Collar owns the authority policy; Quil requests it. |
| **G1 Bell event contract** | Build faults, review requests, and agent task completion fire Bell events. Quil links diagnostics to events. |
| **A3-A8 Lifecycle FSM** | Quil surface follows existing lifecycle states (Visible, Mapped, Minimized, Closing, etc.) through proven LIFECYCLE_TABLE. |

## 8. Future Implementation Phases

| Phase | Scope | Type | Depends On |
|-------|-------|------|------------|
| **H2** | Workstation model (this doc) | Docs | — |
| **H3** | In-memory buffer table | Code (Quil server) | H2 model review |
| **H4** | Buffer list placeholder using existing display primitives | Code | H3, C1-C3 |
| **H5** | Open Linen object into Quil buffer through proven path | Code | H3-H4, H1 (Linen) |
| **H6** | Collar-gated save/build operations | Code | H5, F2 (Collar) |
| **H7** | Mesh diagnostic links | Code | H6, F1 (Mesh) |
| **H8** | Bell build/fault events | Code | H7, G1 (Bell) |

### Phase gate criteria
- **H3 start:** H2 doc accepted, no STOP FIRST triggers hit
- **H5 start:** H1 Linen object model implemented (object storage + retrieval)
- **H6 start:** F2 Collar authority map implemented (grant request/response)
- **H7 start:** F1 Mesh diagnostic model implemented (node creation + linking)
- **H8 start:** G1 Bell event contract implemented (event fire + subscribe)

## 9. STOP FIRST Triggers

Stop and get explicit approval before implementing any of:

| # | Trigger | Reason |
|---|---------|--------|
| 1 | Parser/compiler/build implementation | Out of H2 scope; requires H6+ design |
| 2 | Filesystem/storage implementation | E track owns storage; Quil references Linen |
| 3 | POSIX path assumptions | SexOS has no POSIX; all refs via capability |
| 4 | Kernel edits | STOP FIRST per canon policy |
| 5 | sex-pdx ABI/opcode edits | STOP FIRST per canon policy |
| 6 | New PDX ops | Existing shell display primitives suffice |
| 7 | Authority enforcement/grants | Collar owns policy; Quil requests only |
| 8 | Secret/key handling | Out of scope; requires dedicated design |
| 9 | Renderer-owned editor policy | sexdisplay renders, does not decide |
| 10 | Shared-memory/backing-buffer redesign | Existing PDX surface model unchanged |
| 11 | Cross-PD raw pointers | All inter-server via PDX capabilities |
| 12 | Agent runtime implementation | Out of H2 scope; placeholder only |

## 10. Proof Markers (Future)

Proof markers will be added during implementation phases (H3+):

| Marker | Phase | Trigger |
|--------|-------|---------|
| `[quil.buffer.open]` | H3 | Buffer opened |
| `[quil.buffer.close]` | H3 | Buffer closed |
| `[quil.buffer.switch]` | H3 | Active buffer changed |
| `[quil.edit.request]` | H3 | Edit delta applied |
| `[quil.save.request]` | H6 | Save requested (Collar-gated) |
| `[quil.build.run]` | H6 | Build started (Collar-gated) |
| `[quil.authority.reject]` | H6 | Authority check failed (no-op) |
| `[quil.diagnostic.show]` | H7 | Diagnostic shown in Mesh panel |
| `[quil.bell.link]` | H8 | Bell event linked to diagnostic |
| `[quil.linen.ref]` | H5 | Linen object referenced |
