# LINEN_PROJECT_SCENE_LINK_SPEC_V1

**Status:** PASS — DOCS-ONLY SPEC. No implementation, no code changes.
**Date:** 2026-05-16
**Depends on:** `BELL_LAUNCH_OUTCOME_MARKERS_V1.md` (93-gate baseline), `SCENE_LIFECYCLE_MARKERS_V1.md`.
**Next:** Implementation phases 0–6 (see phase ladder).

---

## 0. PASS/FAIL

**PASS** — DOCS-ONLY SPEC. 0 gates, 0 faults. No code, protocol, or storage changes.

---

## 1. Model Summary

A **ProjectSceneLink** is a metadata-only association between a Linen
project/object and a Silk Scene. It records intent ("this project belongs
to this workspace") without granting any authority, file access, or
rendering capability.

### Core Principles

| Principle | Meaning |
|-----------|---------|
| **Metadata only** | The link is a label, not a capability grant. |
| **No authority** | Linking a project to a scene does not grant file read, app launch, or network access. |
| **Honest about persistence** | durable=0, sync_readback=0. Links are volatile until storage maturity proof. |
| **Separate ownership** | Linen owns objects; Shell owns scenes. Neither crosses the boundary. |
| **Future-aware** | Collar (grants), Mesh (graph), and storage (durability) slot in later. |

---

## 2. Ownership Table

| Component | Role in Project-Scene Link |
|-----------|---------------------------|
| **Linen** | Owns project/object metadata. Stores link records (project_id, scene_id, label). Provides `linen-scene-links` status to querying PDs. |
| **silk-shell** | Owns Scene lifecycle. May display a project badge on the scene chrome (future). Does NOT own Linen objects. |
| **Collar** | Future: approves sensitive capability grants when a project is opened in a scene. Not involved until Phase 4. |
| **Mesh** | Future: visualizes the project→scene→capability graph as nodes/edges. Queries Linen for links, shell for scene state. |
| **sexdisplay** | Renders scene badges/project indicators if shell sends them via surface chrome IPC. Sole framebuffer writer. |
| **Bell** | May receive INFO-lane events when project links are created/modified. Optional. |

---

## 3. V1 Link Model: ProjectSceneLink

```
ProjectSceneLink {
    project_id:    u32,    // Linen object ID
    scene_id:      u8,     // Silk Scene index (0..WORKSPACE_COUNT-1)
    label:         [u8; 64], // Human-readable link name
    status:        u8,     // LinkStatus enum
    persisted:     0,       // V1: volatile RAM only
    durable:       0,       // V1: no storage
    sync_readback: 0,       // V1: no readback
    grants_authority: 0,    // V1: no capability grants
}
```

### Allowed Link States

| State | Value | Meaning |
|-------|-------|---------|
| `suggested` | 0 | Link proposed but not yet confirmed/validated |
| `linked_metadata_only` | 1 | Link exists; only metadata is known (no content access) |
| `status_known` | 2 | Object status is available via SexFiles `object-status` |
| `stale` | 3 | Link exists but object/status may be outdated |
| `blocked_no_readback` | 4 | Link exists but content readback is blocked (durable=0) |

---

## 4. Forbidden Claims

| Claim | Why Forbidden |
|-------|---------------|
| Project link grants file read authority | Linen persist: durable=0, sync_readback=0. No readback proven. |
| Project link grants app launch authority | Launch is shell-only via SLOT_SHELL. Links are passive metadata. |
| Project link grants network authority | Network=0. Collar grants only (Phase 4+). |
| Durable project membership | Storage maturity not reached. Links are volatile. |
| POSIX folders/workspaces | SexOS has no POSIX, no std::fs. Everything is PDX-capability-mediated. |
| Scene owns Linen objects | Shell owns scenes; Linen owns objects. Strict separation. |
| Linen owns Scene focus/layout | Shell is the single focus/layout authority. |
| Automatic project open on scene switch | Scene switch is cosmetic only. Opening a project requires explicit user intent. |
| Cross-scene project migration | V1: links are 1:1 (one project, one scene). Migration is future. |

---

## 5. Phase Ladder

### Phase 0 — THIS SPEC (2026-05-16)
- Docs-only plan.
- Handoff: `docs/handoff/LINEN_PROJECT_SCENE_LINK_SPEC_V1.md`

### Phase 1 — Link Status Markers
- Linen emits `[linen.scene.link]` markers for each known project→scene association.
- Marker-only: no actual link storage, no scene badge rendering.
- Truth marker: `[linen.scene.link.truth] persisted=0 durable=0 sync_readback=0 grants_authority=0`

### Phase 2 — Linen Object Status Link Proof
- Linen queries SexFiles `object-status` for linked projects.
- Proves that object status is accessible without readback.
- Link state transitions from `suggested` → `status_known`.

### Phase 3 — Scene Badge / Status Only
- Shell reads Linen link metadata and displays a small project badge in the scene chrome (top bar).
- Badge is text only (project label), no icon, no thumbnail.
- No pointer/hover/click on badge.

### Phase 4 — Collar Grant-Aware Project Opening
- Collar approves capability grants when a project is opened in a scene.
- Collar checks: is this project linked to this scene? Is the caller authorized?
- Grants are time-bound and revocable.

### Phase 5 — Mesh Graph Visualization
- Mesh queries Linen for project→scene links.
- Mesh queries shell for scene/frame state.
- Renders a graph: project nodes, scene nodes, capability edges.
- Visual only, no interaction.

### Phase 6 — Durable Project/Session Links
- **Requires storage maturity proof** (durable=1, sync_readback=1).
- Links are persisted to SexFiles and survive reboots.
- Session restore can re-open linked projects.

---

## 6. Future Markers

| Marker | Phase | Meaning |
|--------|-------|---------|
| `[linen.scene.link]` | 1 | A project→scene link record exists (marker-only) |
| `[linen.scene.link.truth]` | 1 | Link truth invariant: persisted=0, durable=0, grants_authority=0 |
| `[linen.scene.object.status]` | 2 | Object status available for linked project |
| `[silk.scene.project.badge]` | 3 | Scene chrome shows project badge |
| `[mesh.project.scene.edge]` | 5 | Mesh renders project→scene graph edge |
| `[collar.project.grant.approve]` | 4 | Collar approves project-scoped capability |
| `[linen.scene.link.persist]` | 6 | Link persisted to durable storage |
| `[linen.scene.link.proof.done]` | each | Phase gate complete |

---

## 7. STOP FIRST Boundaries

| # | Boundary | Trigger | Why Blocked |
|---|----------|---------|-------------|
| B1 | Storage protocol change | Adding new SexFiles opcodes for link persistence | Storage maturity: durable=0, sync_readback=0 |
| B2 | Readback claim | Claiming link data is durable or readable from storage | Same reason |
| B3 | Scene switching implementation | Making scene switch depend on project links | Scene switch remains cosmetic until proven |
| B4 | Capability grant from links | Using project→scene link to authorize file/network access | Collar is the grant authority; links are metadata only |
| B5 | Linen owning focus/layout | Linen setting scene focus or frame layout | Shell is single policy authority |
| B6 | Shell owning Linen objects | Shell modifying Linen object metadata | Linen is single object authority |
| B7 | Kernel/sex-pdx/global ABI edits | New syscalls, slots, or opcodes | All servers rebuild |

---

## 8. Handoff Path

```
docs/handoff/LINEN_PROJECT_SCENE_LINK_SPEC_V1.md
```

---

## 9. Commit Command

```bash
git add docs/handoff/LINEN_PROJECT_SCENE_LINK_SPEC_V1.md
git commit -m "docs(linen): Project-Scene link spec V1"
```

---

## 10. References

- `docs/handoff/LINEN_DOCUMENT_LIFECYCLE_PLAN_V1.md` — Linen document model
- `docs/handoff/SCENE_LIFECYCLE_MARKERS_V1.md` — Scene lifecycle (90 gates)
- `docs/handoff/PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` — Storage maturity (durable=0)
- `docs/handoff/BELL_LAUNCH_OUTCOME_MARKERS_V1.md` — 93-gate baseline

---

*End of LINEN_PROJECT_SCENE_LINK_SPEC_V1.md*
