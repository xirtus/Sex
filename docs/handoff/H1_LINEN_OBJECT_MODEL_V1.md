# H1: Linen Object Model — Spec

**Status:** Approved (Docs/Model only)
**Commit:** *(pending)*
**Build:** N/A (no code changes)

## 1. Purpose

Linen is SexOS's **object/project workspace**. It presents user-owned objects,
projects, documents, references, and capability-scoped views. Linen is **not
just a file browser** — it is the object model that sits above storage, above
filesystems, and above authority.

### What Linen is

- **Object workspace.** Linen organizes user data as objects with identity,
  kind, relationships, and lifecycle — not as POSIX files with paths.
- **Capability-scoped.** Every object reference can carry a grant/Collar token.
  Opening an object requires authority.
- **View-driven.** Linen presents objects through views: project view, recent
  objects, open in scene, grant status.
- **Shell-integrated.** Linen objects surface through Scene/Frame/Tab (D1 path),
  Atlas (C1), and later Mesh/F2 visualization.

### What Linen is NOT in H1

- **Not a raw filesystem browser.** No directory tree, no POSIX paths, no
  `/home/user` conventions. Objects are identified by `object_id`, not path.
- **Not storage implementation.** No block device, no disk format, no data
  persistence. H1 defines the object model only.
- **Not authority owner.** Collar (F2) governs all grant decisions. Linen asks
  Collar before opening, creating, deleting, or sharing objects.
- **Not a renderer.** Linen produces object state, not pixels. silk-shell and
  sexdisplay handle rendering through existing 0xEC/0xEF/0xEE primitives.

## 2. Object Types

Every Linen object has an `object_kind` that determines its semantics,
available operations, and display treatment.

| Object Kind | Description | Example |
|-------------|-------------|---------|
| `Project` | Top-level container for related objects | "SexOS Kernel", "My Game", "Tax Documents" |
| `Document` | Human-readable content document | Design doc, specification, notes |
| `CodeFile` | Source code file | `main.rs`, `lib.rs`, `build.rs` |
| `MediaAsset` | Image, audio, or video asset | Icon, screenshot, recording |
| `BuildArtifact` | Compiled output or binary | ISO, ELF, WASM module |
| `Folder` | Collection of objects (not POSIX dir) | "Screenshots", "Drafts", "Archive" |
| `Reference` | Link to an object in another project | Cross-project document ref |
| `DeviceImport` | Placeholder for device-originated data | Photo import, USB file transfer |
| `BellEventRef` | Reference to a Bell event linked to this object | Build failure notification |
| `QuilWorkspaceRef` | Reference to a Quil workspace/project | "Open in Editor" |
| `MeshDiagRef` | Reference to a Mesh diagnostic for this object | Authority graph for this object |

## 3. Object Fields

Every Linen object is a fixed-size record with the following fields:

| Field | Type | Description |
|-------|------|-------------|
| `object_id` | u64 | Unique object identifier |
| `object_kind` | enum (see §2) | Type of this object |
| `display_name` | `[u8; 64]` | Human-readable name (UTF-8, zero-padded) |
| `parent_id` | u64 | Parent folder/project ID, or `0` for root |
| `project_id` | u64 | Project this object belongs to |
| `owner_identity` | u64 | User/PD identity that owns this object |
| `grant_ref` | u64 | Collar grant reference, or `0` if public |
| `privacy_class` | enum | `Public`, `Internal`, `Confidential`, `Secret` |
| `dirty` | bool | Unsaved changes flag |
| `lifecycle_state` | enum | `Allocated`, `Loaded`, `Modified`, `Saved`, `Archived`, `Deleted` |
| `thumbnail_icon` | u8 | Icon token index (0 = default, 1..N = specific icon). No image cache. |
| `open_scene` | u8 | Scene index where this object is open, or `0xFF` |
| `open_frame` | u32 | Frame ID where this object is open, or `0` |
| `open_tab` | u8 | Tab index where this object is open, or `0xFF` |
| `proof_marker` | `[u8; 16]` | Audit proof marker for tracking |

## 4. Views

Linen presents objects through these views. Each view is a potential surface
that can be rendered through the Scene/Frame/Tab model.

| View | Description | Future implementation |
|------|-------------|---------------------|
| **Project view** | Tree/grid of objects within a project | H3 placeholder list UI |
| **Recent objects** | Chronological list of recently accessed objects | H3 |
| **Open in Scene** | Objects currently open in frames/tabs (from Scene/Frame/Tab model) | H4 open object as tab |
| **Object details** | Metadata, grant status, linked references for a single object | H4 |
| **Grant status** | Collar grant state for an object (who can read/write/share) | H5 Collar-gated ops |
| **Search** | Search/filter placeholder (no real search in H1) | Future |
| **Import inbox** | Incoming objects from devices, network, or other users | Future |

## 5. Operations

Operations on Linen objects. Authority-sensitive ops require Collar approval
before execution.

| Operation | Authority-sensitive | Collar gate |
|-----------|-------------------|-------------|
| `open_object(id)` | Yes | Requires read grant |
| `close_object(id)` | No | — |
| `select_object(id)` | No | — |
| `rename_request(id, name)` | Yes | Requires write grant |
| `create_request(kind, name, parent)` | Yes | Requires create grant in project |
| `delete_request(id)` | Yes | Requires delete/archive grant |
| `archive_request(id)` | Yes | Requires archive grant |
| `reveal_grant_status(id)` | No | Reads existing grant ref |
| `send_to_quil(id)` | Yes | Requires read grant + Quil workspace grant |
| `show_in_mesh(id)` | No | Diagnostic only |
| `share_request(id, target_pd)` | Yes | Requires share grant + target receive grant |

**Non-authority operations** (always allowed): view metadata, see grant status,
see open-in-scene state.

## 6. Invariants

1. **Never bypass Collar.** All authority-sensitive operations (open, create,
   delete, rename, share, send to Quil) must go through Collar for grant
   approval before execution.

2. **No raw cross-PD pointers.** All object references are `object_id` (u64)
   values. No kernel object handles, no memory addresses.

3. **Never writes framebuffer.** Linen produces object state, not pixels.
   Rendering is done by silk-shell using existing 0xEC/0xEF/0xEE primitives.

4. **No POSIX assumptions.** Objects are identified by `object_id`, organized
   by `parent_id` and `project_id` — never by path strings. No `/`, no `..`,
   no CWD, no file extensions.

5. **No filesystem/storage required in H1.** The object model exists in memory
   in H1. Persistence, block devices, and disk formats are deferred.

6. **Safe degradation.** If a referenced object's parent, project, or grant
   record is missing, the viewer shows "object unavailable" rather than
   faulting.

7. **Renderer never owns object policy.** sexdisplay renders Linen surface
   pixels under shell authority. It never decides object access, visibility,
   or grant state.

## 7. Relationship to Existing Work

| Component | Relationship to Linen |
|-----------|----------------------|
| **D1/D2** | Linen placeholder lifecycle (surface 200, frame 2) — proves Scene/Frame/Tab attach path |
| **B1-B4** | Frame/tab attach, tiling (B3), chrome (B4) — Linen uses same model |
| **C1-C3** | Atlas snapshot — Linen objects visible in Atlas when open in a scene |
| **F1 Mesh** | Can visualize object links, object→surface→frame→scene relationships later |
| **F2 Collar** | Governs all authority-sensitive object operations |
| **G1 Bell** | Can reference Linen objects in events (e.g., "Build complete for object X") |
| **Quil (future)** | Opens/edits Linen CodeFile and Document objects |
| **sexstore** | Future storage backend for object data |

## 8. Future Implementation Plan

### H1 (this document)
- ✅ Docs/model definition
- No implementation

### H2 — Quil Workstation Model (docs)
- Define Quil editor/workspace model (separate doc)
- CodeFile editing, build, review semantics

### H3 — Linen In-Memory Object Table
- Add fixed-size object table in silk-shell or linen PD
- `LINEN_OBJECTS: [Option<LinenObject>; 64]` — static ring
- Object lifecycle: Allocated → Loaded → (Modified) → Saved → Archived → Deleted
- Open/close object tracking
- **Requires STOP FIRST review**

### H4 — Linen Placeholder List UI
- Render object list using existing 0xEC/0xEF/0xEE primitives
- Text line rendering using `QUIL_PLACEHOLDER_COLOR`-style blocks
- No real icons, no thumbnails, no images — color-coded bars only
- Browser keybindings matched to object operations

### H5 — Open Object as Tab/Frame
- Open a Linen object as a tab through the proven D1 path
- Attach to active scene FRAMES, tile, focus
- Object metadata displayed in tab chrome
- **Requires STOP FIRST review**

### H6 — Collar-Gated Operations
- Wire object operations through Collar grant checks
- Grant-denied operations return `[linen.object.reject.unauthorized]`
- **Requires Collar implementation first**
- **Multiple STOP FIRST reviews required**

### H7 — Mesh Object Graph
- Visualize Linen objects and their relationships in Mesh
- Object → surface → frame → scene → grant edges
- **Requires Mesh placeholder surface (I1) first**

## 9. STOP FIRST Triggers

Stop all Linen work and escalate if any of the following are required:

- **Filesystem/storage implementation** — H1 is model only; storage is future
- **POSIX path assumptions** — Linen never uses paths; object_id only
- **Kernel edits** — Linen is userspace only
- **`crates/sex-pdx/` ABI/opcode edits** — requires contract + STOP review
- **New PDX ops** — requires contract + STOP review
- **Authority enforcement/grants** — Collar owns this, not Linen
- **Secret/key handling** — Collar owns secrets
- **Renderer-owned object policy** — sexdisplay never decides access
- **Shared-memory/backing-buffer redesign** — Uses existing PDX/display path
- **Cross-PD raw pointers** — Never stored or transmitted
