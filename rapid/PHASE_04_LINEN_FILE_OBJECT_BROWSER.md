# PHASE 04: Linen — The Object Layer (User's Semantic Graph Over SexOS Objects)

## Revolutionary Vision

Every operating system in history has had a file manager that shows **path → bytes**. Finder, Dolphin, Nautilus, Explorer — they all display a hierarchical filesystem tree where files are dead icons and their meaning lives only in the user's head.

**Linen is not a file manager. Linen is the object layer.**

In a SASOS / SAASOS, object identity is stable and direct. There is no need to fake everything through POSIX paths and process-local addresses. The OS can say: objects are first-class citizens. A file, notification, build log, screenshot, AI prompt, trace, package, project, or capability reference can become **something visible, saved, linked, pinned, searched, moved, and reasoned about** — all within one object workspace.

```
old_file_manager = "folders and files as dead icons.
                    apps own their own histories.
                    notifications disappear.
                    projects are manually reconstructed.
                    permissions are hidden.
                    context is scattered across apps."

linen_answer = "objects become first-class OS citizens.
               a file, notification, build log, screenshot,
               prompt, trace, package, project, or capability
               reference can become something visible, saved,
               linked, pinned, searched, moved, and reasoned about."
```

### Linen in the SexOS Canon

The naming doc places Linen directly beside the core primitives:

| Primitive | Role |
|-----------|------|
| Silk | Desktop shell — windows, scenes, frames, focus |
| Scene | Layout and theme state per workspace |
| Atlas | Overview / Exposé |
| Frame | Tiled container for surfaces |
| Tab | App surface within a frame |
| **Linen** | **Files, objects, projects — the object layer** |
| Mesh | Capability and service graph |
| Collar | Authority, grants, secrets, trust |
| Quil | Language, code, document editing |

**Boundaries are strict:**

| System | Owns | Does NOT Own |
|--------|------|-------------|
| **Silk** | windows, scenes, frames, focus, tiling, gestures, frame borders, shell policy | objects, files, storage |
| **Linen** | files, folders, project spaces, saved objects, object history, attachments, pinned references, project context | tiling, shell tabs, workspace atlas, frame borders, global focus |
| **Mesh** | live authority/topology graph — what exists, PDs, devices, PDX edges, caps, failures | user objects, grants, secrets |
| **Collar** | grants, secrets, identities, revocation | objects, topology |
| **sexfiles** | VFS/storage service layer | user-facing object workspace |
| **sexdisplay** | pixel rendering | object model, shell policy |

### What Makes Linen Revolutionary

**1. Objects, not files.** A "file" is just bytes at a path. A Linen object is:

```
Object {
    id: ObjectId,              // stable, direct — SAASOS advantage
    name: FixedStr<64>,
    kind: ObjectKind,           // Document, Code, Image, Log, Prompt, Trace, ...
    source_pd: PdId,            // who created it
    storage_cap: Capability,    // who can read/write the bytes
    read_cap: Capability,       // who can read the object metadata
    write_cap: Capability,      // who can modify the object
    project_context: ProjectId, // which project it belongs to
    related_events: EventRefs,  // Bell notifications, build events, faults
    related_builds: BuildRefs,  // build logs that reference this object
    related_prompts: PromptRefs,// AI prompts that generated or analyzed this object
    trust_metadata: TrustInfo,  // provenance chain, signature status
    created_at: Ticks,
    modified_at: Ticks,
}
```

**2. The living project graph.** A project in Linen is not a directory. It is a **semantic graph**:

```
/projects/SexOS
  ├── code files              (typed: SourceCode, language: Rust)
  ├── docs                    (typed: Document, rendered in Quil)
  ├── handoffs                (typed: DesignDoc, linked to commits)
  ├── build logs              (typed: BuildLog, linked to code objects)
  ├── failed QEMU traces      (typed: Trace, linked to build logs)
  ├── Claude/Codex prompts    (typed: Prompt, linked to objects they generated)
  ├── capability maps         (typed: CapabilityGraph, snapshot from Mesh)
  ├── PD fault history        (typed: FaultLog, linked to Collar revocations)
  ├── screenshots             (typed: Image, linked to build commits)
  └── pinned Bell events      (typed: Notification, saved from Bell)
```

Every object type knows what it can do. A `SourceCode` object knows it can be compiled. A `Trace` object knows it can be replayed. A `Prompt` object knows it was used to generate other objects. The graph carries semantics, not just hierarchy.

**3. Capability-aware visibility.** Linen shows only objects the user has capabilities to. The set of visible objects IS the set of accessible object capabilities. If you can't see an object, it doesn't exist — not grayed out, not "access denied" — invisible. This makes the capability model tactile: users experience it directly.

**4. Linen reads through PDX, not shared memory.** Linen does not directly read anyone else's memory. Cross-domain pointers hardware-trap. Linen asks through PDX and receives only authorized object metadata or lent-memory capabilities:

```
Linen_Read_Flow:
  Linen UI requests object
    → PDX to sexfiles / sexstore
    → capability check (via Collar)
    → authorized metadata returned (or zero-copy handover via capability slot)
    → Linen receives bounded object view
    → sexdisplay renders Linen surface
```

**5. Bell notifications become Linen objects.** Notifications are not disposable popups. They can be saved to a project, attached to a file/task, pinned to a workspace, turned into a reminder, inspected for sender/action capabilities, or replayed as event trails. They become part of the user's **project memory**.

**6. Object persistence + capability awareness = the combination that makes Linen more than Finder.** Finder shows storage. Linen shows **meaning**. A project folder is not just a directory — it's a **living project graph** with code, docs, builds, traces, prompts, capabilities, faults, screenshots, and notifications all linked together in one object workspace.

**7. Stable object identity (SAASOS advantage).** Because SexOS is a single address space OS, object identity can be direct and stable across the entire system. An `ObjectId` refers to the same object from any PD. No need for network paths, mount points, or cross-filesystem references. The object ID is a first-class type across the entire system.

---

## Ownership

- **Linen** (exclusive): object workspace surface, project graph, object cards, object-type dispatcher
- **sexfiles** (exclusive): object storage — CRUD operations on objects, capability-gated access control
- **sexstore** (future): persistent object storage across reboots
- **Collar** (integration): capability checks on object access, grant/revoke object capabilities
- **Mesh** (integration): objects appear as nodes in the living system graph
- **Bell** (integration): notifications can be saved as Linen objects
- **Quil** (consumer): opens Document/SourceCode objects from Linen
- **sexdisplay** (renderer): renders the Linen surface (standard surface chrome)
- **silk-shell** (integration): Linen surface lifecycle, open-file dispatch, chrome frame

## What Already Exists
- Linen server binary exists (boots, listens on PDX slot) but object workspace surface is not built
- sexfiles defined in manual as PDX VFS with open/read/write/close/stat/readdir
- sexstore planned for Phase 11 (persistence)
- Silk-shell has surface creation (0xEC), focus management, open-file dispatch patterns
- No object model, no object types, no project graph, no capability-gated object access exists
- No integration with Bell (notifications), Mesh (object graph nodes), or Collar (capability checks)

## Bundle

### Core Object Model (must exist before any UI)

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| `ObjectKind` enum | Document, SourceCode, Image, Log, Prompt, Trace, Notification, AppManifest, Collection, CapabilityRef | 2h | HIGH |
| `ObjectMeta` struct | id, name, kind, source_pd, project_id, created_at, modified_at, related refs (fixed-size arrays) | 2h | HIGH |
| `ObjectId` type | u64 — sequential in V1, hash-prefix in V2. Opaque to consumers. | 1h | HIGH |
| sexfiles object CRUD | `OP_OBJECT_CREATE`, `OP_OBJECT_READ_META`, `OP_OBJECT_READ_DATA`, `OP_OBJECT_WRITE_DATA`, `OP_OBJECT_LIST` — all capability-gated | 8h | HIGH |
| Capability-gated access | sexfiles checks caller PD identity against object's `read_cap`/`write_cap` before returning data. No capability → empty response (not error). | 4h | HIGH |

### Linen Surface

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| Linen PDX server | Boot, listen, serve object workspace surface | 3h | HIGH |
| Object browser view | Objects grouped by kind, displayed as colored cards — not a directory tree | 6h | HIGH |
| Project graph view | Objects displayed in a project context with typed links between them | 6h | Medium |
| Object metadata card | Click object → see: name, kind, source, size, created, modified, related events/builds/prompts | 3h | Medium |
| Object-type dispatcher | Click opens object in capability-authorized viewer (source→Quil, image→sexdisplay, notification→Bell) | 4h | Medium |
| Collection navigation | "Show all objects of kind SourceCode in project SexOS" — query-based, not tree-based | 3h | Medium |
| Open in Quil | Transfer capability → Quil reads object data via sexfiles → Quil renders | 3h | Low (depends on Phase 5) |

### Object → System Integration

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| Bell → Linen save | Save a Bell notification as a Linen object (kind: Notification, linked to project) | 3h | Medium |
| Mesh object nodes | Objects appear as typed nodes in Mesh graph: `GraphNode { kind: Object, id, label }` | 2h | Low |
| Collar capability check | sexfiles queries Collar: "Does Pd(silk-shell) have ReadCap for Object(47)?" | 2h | Low (after Phase 6) |

## Smallest First Step
**Create one object and read it back.** sexfiles stores `Object { id: 1, name: "hello", kind: Document, source_pd: LINEN, data: "Hello, SexOS" }`. Linen queries `OP_OBJECT_LIST { kind: Document }` and receives object 1. Linen renders a colored card: "hello (Document)". Nothing else.

This proves:
- Object model compiles and is fixed-size
- sexfiles stores and retrieves objects
- Capability gate works (query from unauthorized PD returns empty)
- Linen can render object cards from sexfiles data
- The entire pipeline: Linen → PDX → sexfiles → capability check → PDX → Linen → sexdisplay

## Dependencies
- **Blocking**: None — object model is independent of shell, input, display contract
- **Blocked by**: Phase 6 (Collar) for full capability integration; Phase 11 (sexstore) for persistence
- **Can parallelize with**: Phase 3, Phase 5 (Quil can design object types for SourceCode/Document before Linen exists)
- **Key insight**: Linen works WITHOUT persistence, WITHOUT Collar, and WITHOUT Mesh. The object model + sexfiles CRUD + surface rendering is sufficient for V1. Persistence, capability integration, and graph visibility are additive layers.

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Object model is too abstract ("what IS an object?") | Medium | High | V1: Limit to 4 concrete kinds: Document, SourceCode, Image, Collection. Other kinds (Notification, Prompt, Trace) added when their source systems exist. |
| sexfiles doesn't exist as a running server | High | HIGH | Start with Linen-hosting objects in-memory. sexfiles integration is the second step, not the first. Hardcoded object list proves the surface before storage exists. |
| Capability-gated access is complex (need Collar) | Medium | Medium | V1: sexfiles checks a hardcoded access list (which PDs can access which objects). Collar integration deferred to Phase 6. The architecture is the same — just the policy source changes. |
| Object types proliferate (every system wants its own kind) | High | Low | Fixed enum (max 16 kinds). New kinds require code change, not configuration. This is intentional for a no_std system — no dynamic type registration. |
| Project graph view is too complex for V1 | Medium | Low | V1: Flat list grouped by kind. "Project" is a tag, not a graph. Graph view (linked objects, provenance chains) deferred to V2. The object model supports it, but V1 doesn't render it. |
| Linen surface is slow with many objects | Low | Medium | Fixed max visible objects (128). Pagination via "next page" PDX query. sexfiles returns up to 32 objects per query. No unbounded rendering. |

## Revolutionary Design Details

### The Object Model (V1)

```rust
/// Every first-class entity in the user's workspace is an Object.
/// Fixed-size, no_std-safe, no heap.
#[derive(Clone, Copy)]
#[repr(C)]
struct Object {
    id: u64,
    name: FixedStr<64>,          // user-visible name
    kind: ObjectKind,            // type tag — determines viewer, icon, behavior
    source_pd: u32,              // which PD created this object
    project_id: u64,             // which project this belongs to (0 = unassigned)
    size: u32,                   // data size in bytes
    created_at: u64,             // ticks
    modified_at: u64,            // ticks
    read_cap: u32,               // capability slot for read access
    write_cap: u32,              // capability slot for write access
    related_count: u8,           // how many related-object references follow
    related: [u64; 8],           // up to 8 related object IDs
    _reserved: [u8; 32],         // future: provenance chain, trust metadata
}

#[repr(u8)]
enum ObjectKind {
    Document = 1,       // editable in Quil
    SourceCode = 2,     // editable in Quil, buildable
    Image = 3,          // viewable in sexdisplay
    Collection = 4,     // group of objects (replaces "directory")
    Notification = 5,   // saved from Bell
    Prompt = 6,         // AI prompt that generated artifacts
    Trace = 7,          // execution trace, fault log
    BuildLog = 8,       // build output, linked to SourceCode
    AppManifest = 9,    // app package definition
    CapabilityRef = 10, // reference to a capability (from Collar/Mesh)
}
```

**Size: 64 + 4 + 4 + 4 + 8 + 8 + 4 + 4 + 1 + 64 + 32 = ~197 bytes. Fits in 3 PDX messages.**

### The Linen Surface

Linen renders as a standard SexOS surface (0xEC → focused → renders via sexdisplay). Its chrome is standard (Frame Lights + top bar + tab strip). Its content area shows the object workspace.

The Linen surface is **not** a file browser tree. It is a **query-results canvas**:

```
┌─────────────────────────────────────────────────────────────────────────┐
│ linen://projects/SexOS/                            [grid] [list] [graph] │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐       │
│  │  docs/     │  │  src/      │  │  handoffs/ │  │  traces/   │       │
│  │  24 objects│  │  47 objects│  │  12 objects│  │  8 objects │       │
│  └────────────┘  └────────────┘  └────────────┘  └────────────┘       │
│                                                                         │
│  Recent: [handoff_06.md] [mesh_phase.md] [build_1203.log] [trace_47]   │
│  Related Bell: [build failed at T+3400] [capability revoked: App A]    │
│  Related Prompts: [refactor mesh query] [add object provenance]        │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  Object: handoff_06.md                                          │  │
│  │  Kind: Document  |  Size: 12KB  |  Source: Quil                  │  │
│  │  Created: T+1200  |  Modified: T+3400  |  Project: SexOS        │  │
│  │  Related: build_1203.log (linked), trace_47 (linked)             │  │
│  │  Capabilities: Read(user) Write(user) Share(Quil)               │  │
│  │  [OPEN IN QUIL]  [SHARE]  [PIN TO PROJECT] [SAVE BELL EVENT]   │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

**Key UX differences from a file manager:**
- Top bar shows an object query (`linen://projects/SexOS/`), not a filesystem path
- Objects are grouped by **kind**, not by directory
- Object cards show **typed metadata** (source PD, capabilities, related objects)
- The metadata panel shows **related Bell events, build logs, and AI prompts** — not just file dates
- The "OPEN IN" button routes to the type-appropriate viewer, not a file extension mapping
- "SAVE BELL EVENT" lets the user pull a notification into the project as a permanent object

### Object Read Flow (Zero-Copy)

```
User clicks "handoff_06.md" in Linen
  → Linen sends OP_OBJECT_READ_META { id: 47 } to sexfiles
    → sexfiles checks: does Linen's PD have read_cap for object 47?
      → Yes: return ObjectMeta { name, kind, size, source_pd, ... }
      → No: return empty (object doesn't exist from caller's perspective)
  → Linen renders metadata card
  → User clicks "OPEN IN QUIL"
    → Linen sends OP_OBJECT_READ_DATA { id: 47 } to sexfiles
      → sexfiles checks read_cap again
        → Yes: return data (up to 98 bytes in PDX payload) or capability slot for large objects
        → No: return empty
    → Linen transfers capability to Quil via PDX
    → Quil reads object data via sexfiles (same capability check)
    → sexdisplay renders Quil surface with content
```

**Every access goes through the capability gate. No object is ever directly memory-mapped across domains. The SAASOS advantage is stable object identity, not casual pointer sharing.**

### Object → System Integration Points

**With Bell:**
```
Bell notification arrives (Phase 9)
  → User clicks "Save to Linen" on notification
  → Bell sends OP_OBJECT_CREATE { 
        name: "build failed at T+3400", 
        kind: Notification, 
        data: notification_payload,
        project_id: "SexOS"
    } to sexfiles
  → sexfiles returns Object { id: 142 }
  → Linen refreshes project view → shows new Notification object
```

**With Mesh:**
```
sexfiles creates object 47
  → sexfiles pushes OP_MESH_PUSH_NODE { kind: Object, id: 47, label: "handoff_06.md" }
  → Mesh records object node in temporal graph
  → Quil Mesh panel shows object as a graph node connected to its project
```

**With Collar:**
```
Quil requests read access to object 47
  → sexfiles checks: does Quil have read_cap for object 47?
  → No → sexfiles queries Collar: "Can Quil have read access to object 47?"
  → Collar checks: user policy, pattern match, prompt if needed
  → Collar grants: Capability { resource: Object(47), operations: Read, lifetime: 1h }
  → sexfiles caches the grant
  → Quil can now read object 47
```

## Exit Criteria (Done Checklist)

**Core Object Model:**
- [ ] `ObjectKind` enum defined (at least 4 kinds: Document, SourceCode, Image, Collection)
- [ ] `Object` struct defined (fixed-size, repr(C), no_std-safe)
- [ ] sexfiles CRUD opcodes implemented: create, read_meta, read_data, write_data, list
- [ ] sexfiles capability gate: unauthorized PD receives empty response (not error, not data)
- [ ] Hardcoded access list works before Collar integration

**Linen Surface:**
- [ ] Linen PDX server boots, creates surface, renders object cards
- [ ] Objects displayed grouped by kind (not directory tree)
- [ ] Click object → metadata card shown with name, kind, size, source, dates
- [ ] Click "OPEN IN" → dispatches to type-appropriate viewer (Quil for Document/SourceCode)
- [ ] Collection navigation works: "show all SourceCode in project SexOS"
- [ ] Project context: objects show their project, filterable by project

**Integration:**
- [ ] Bell notification can be saved as Linen Object (kind: Notification)
- [ ] Saved notification appears in project view
- [ ] Mesh object nodes: objects appear in Mesh graph (if Phase 6 complete)
- [ ] Collar capability check: sexfiles queries Collar for object access (if Phase 6 complete)
- [ ] Build passes. Boot passes. No panic.
- [ ] Only linen, sexfiles, sex-pdx changed. No kernel, no sexdisplay protocol changes.

## Testing Strategy

- **Object model**: Create objects of each kind, verify fields round-trip correctly. Verify fixed-size (assert size_of::<Object>() == expected).
- **Capability gate**: Create object with read_cap for LINEN only. Query from silk-shell (unauthorized). Verify empty response. Query from Linen (authorized). Verify full response.
- **Linen surface**: Boot QEMU. Verify Linen surface created and focusable. Verify objects displayed as colored cards grouped by kind.
- **Object routing**: Click Document object → verify Quil surface spawns with document data. Click Image → verify sexdisplay shows image.
- **Bell integration**: Send test notification via Bell. Click "Save to Linen." Verify notification appears as object in Linen.
- **Stress**: Create 128 objects. Verify Linen displays all (paginated). Verify no frame drops.
- **Regression**: All existing shell/display/input markers fire at expected counts.

## Efficiency Opportunity

**The biggest time save is recognizing that the object model IS the value.** The Linen surface (visual browser) is secondary. If the object model with capability-gated access is implemented first, every other system (Bell, Mesh, Collar, Quil) can integrate with it before Linen has a fancy UI. The surface can be a minimal grid of colored blocks — the architecture is what matters.

**V1 should ship:**
1. sexfiles object CRUD with capability gate
2. Minimal Linen surface showing objects as colored cards
3. One integration: save Bell notification as object
4. Everything else deferred to V2

This gets the revolutionary architecture in place with minimal surface area. The pretty project graph view comes later.

## Completeness Gain
User objects/projects: **15–25% → 55–65%** (object model + capability gate + minimal surface). **15–25% → 40–50%** (surface only, no object model). **Recommendation**: build the object model first — it unlocks every other system.

## Files Changed
- `servers/sexfiles/src/main.rs` (object CRUD opcodes, capability gate, `ObjectKind`, `Object` struct)
- `servers/linen/src/main.rs` (object workspace surface, object card rendering, query-based navigation, type dispatcher)
- `servers/silk-shell/src/main.rs` (Linen surface lifecycle, open-object dispatch)
- `servers/bell/src/main.rs` (save notification as Linen object — if Phase 9 integrated)
- `crates/sex-pdx/src/lib.rs` (OP_OBJECT_CREATE, OP_OBJECT_READ_META, OP_OBJECT_READ_DATA, OP_OBJECT_WRITE_DATA, OP_OBJECT_LIST — all in free opcode range)
- `servers/quil/src/main.rs` (open Document/SourceCode objects from Linen — if Phase 5 integrated)

## Forbidden
- POSIX filesystem semantics (no paths, no mount points, no inodes)
- Shared-memory object handover (capability slot transfer only)
- Direct memory reads across domains (PDX only — cross-domain pointers trap)
- Tree-only navigation (query-based navigation is primary; tree is one type of query)
- Object type proliferation beyond 16 kinds (fixed enum — no dynamic registration)
- Dynamic allocation in object storage (fixed-size arrays in sexfiles)
- Kernel changes
- sexdisplay protocol changes

## Relationship to Other Systems

```
User sees:    Linen surface ── objects grouped by kind, project context, related events
              │
Backed by:    sexfiles ── object CRUD with capability-gated access
              │
Protected by: Collar ── capability grant/revoke/decay for object access
              │
Visible in:   Mesh ── objects as graph nodes with typed edges
              │
Created by:   Quil (documents), Bell (notifications), shell (screenshots), build (logs)
              │
Consumed by:  Quil (edits), sexdisplay (views), Linen (browses), Mesh (observes)
```

## Next Phase
PHASE_05_QUIL_LANGUAGE_WORKSTATION.md

## Parallel Note
Phase 4 (Linen) and Phase 5 (Quil) should be developed together — they share the object model. Quil creates Document and SourceCode objects. Linen displays them. The object type dispatcher (Linen → Quil) is the integration point. Both can start independently (Linen with hardcoded objects, Quil with direct file access) and converge when the object model is stable.
