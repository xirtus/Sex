# SCENE_PERSISTENCE_PLAN_V1

**Status:** Plan only. No implementation.

**Scope warning:** SCENE_PERSISTENCE_PLAN_V1 persists **shell layout intent only**. It does not relaunch apps, restore process/PD state, resurrect SurfaceIds, restore app memory, or grant capabilities. Scene persistence ≠ Session restore. Scene persistence restores shell layout containers. Session restore (Track B) later uses app identity, manifests, documents, and capability validation.

**Prerequisites:** Phase 01 contract gates complete, Phase 02 Scene/tombstone/frame/tab complete, TILING_ENGINE_V1 complete, CHROME_TEMPLATE_V1 complete. Track A (COMPOSITOR_LIFECYCLE) A1–A4 must be complete before restore validates surface liveness.

---

## 1. Mission

Design the smallest safe persistence plan for Silk Scene/Frame/Tab state — persist active Scene, frame layout/geometry, tab stack ordering, minimized/collapsed frames, and restore a deterministic shell layout on boot.

No filesystem redesign. No sexstore/sexfiles capability changes. No PDX ABI edits. No kernel changes. No app relaunch. No session restore.

---

## 2. Context — Existing Infrastructure

### Shell state already present (`servers/silk-shell/src/main.rs`)

| Symbol | Type | Description |
|--------|------|-------------|
| `FRAMES` | `[Option<ShellFrame>; MAX_FRAMES]` | static array of frame objects (MAX_FRAMES=4) |
| `ShellFrame` | struct | frame_id, active_tab, tab_count, tabs[8], scene_id, flags, normal_x/y/w/h |
| `ShellTab` | struct | surface_id, title_id, flags |
| `ACTIVE_SCENE_IDX` | `u8` | active workspace/scene index |
| `FRAME_FLAG_MINIMIZED` | `u32 = 1<<0` | frame minimized flag |
| `FRAME_FLAG_ZOOMED` | `u32 = 1<<1` | frame zoomed flag |
| `FRAME_FLAG_TOP_BAR` | `u32 = 1<<2` | frame top bar chrome flag |
| `TOMBSTONES` | `[u64; 8]` | circular tombstone list for closed surface IDs |
| `SURFACE_*` vars | various | per-surface position/size/alive tracking |
| `tile_visible_frames()` | fn | deterministic tiling after layout changes |

### Existing PDX call pattern

- **sexstore K/V service** (slot 10, `SLOT_SEXSTORE`)
- Opcodes: `OP_KV_GET = 0xB0`, `OP_KV_PUT = 0xB1`
- Values are `u64` blobs (8 bytes)
- Existing usage: `SCENE_SETTINGS_KEY_APPEARANCE = 0x01` — packs `preset_idx, chrome_flags, accessibility_flags` into a u64
- Pattern: fire-and-forget PUT on setting change, asynchronous GET at boot with deferred reply handling

**Note:** The PDX call pattern may be proven for single-value K/V. Durable correctness, corruption recovery, and replay semantics are **not** proven by that alone. V1b storage-backed persistence cannot assume sexstore provides these guarantees without explicit design.

---

## 3. What Should Persist

### Per-session snapshot

| Field | Type | Source | Notes |
|-------|------|--------|-------|
| magic | u8 | constant `0xAC` | data integrity marker (V1a debug only) |
| version | u8 | constant `0x01` | schema version for future evolution |
| generation | u8 | monotonic counter | SnapshotGeneration — shell-local ordering metadata; detects older in-memory candidates within same boot, NOT a durability/security guarantee |
| active_scene_id | u8 | `ACTIVE_SCENE_IDX` | which workspace is active |
| frame_count | u8 | count of `Some` frames | must be <= MAX_FRAMES |
| padding | u8[3] | zero | reserved |

### Per-frame record (repeat N times where N = frame_count)

| Field | Type | Source | Notes |
|-------|------|--------|-------|
| frame_id | u8 | `ShellFrame.frame_id` | restore identity for new shell objects |
| scene_id | u8 | `ShellFrame.scene_id` | scene membership |
| flags | u32 | `ShellFrame.flags` | minimized/zoomed/top_bar bits |
| normal_x | i32 | `ShellFrame.normal_x` | pre-zoom geometry |
| normal_y | i32 | `ShellFrame.normal_y` | pre-zoom geometry |
| normal_w | u32 | `ShellFrame.normal_w` | pre-zoom geometry |
| normal_h | u32 | `ShellFrame.normal_h` | pre-zoom geometry |
| active_tab | u8 | `ShellFrame.active_tab` | which tab is active |
| tab_count | u8 | `ShellFrame.tab_count` | number of valid tabs |
| tabs | `[TabEntry; MAX_TABS_PER_FRAME]` | per-frame tab array | 8 entries in V1 |

### Per-tab entry (within each frame)

| Field | Type | Source | Notes |
|-------|------|--------|-------|
| app_hint | u64 | `ShellTab.surface_id` | historical/app-hint only — NOT a live SurfaceId restore target |
| title_id | u64 | `ShellTab.title_id` | reserved |
| flags | u32 | `ShellTab.flags` | reserved |

### Serialized layout

Total size (max): header(8) + MAX_FRAMES(4) * [frame_header(28) + MAX_TABS_PER_FRAME(8) * tab_entry(16)] = 8 + 4*(28 + 8*16) = 8 + 4*156 = 632 bytes

This exceeds a single u64 (8 bytes). Therefore:

**V1a approach (RECOMMENDED): shell-local static snapshot only.** Store the snapshot in a static buffer in shell memory. No storage writes. Restore is from RAM only — lost on reboot. Creates **new shell objects**, never reuses old live object IDs unless current shell model explicitly supports safe reuse.

**V1b (future, storage-backed):** Blocked until corruption handling, partial-read behavior, and deterministic replay failure modes are specified. See §7 for explicit requirements.

---

## 4. What Should NOT Persist

| Item | Reason |
|------|--------|
| Transient hover/drag state | Ephemeral interaction, not scene state |
| Any prior SurfaceId as live restore target | Surface IDs are historical/debug references only; never resurrected |
| Tombstone list | Debug artifact, not scene state |
| Raw pointer/cross-PD memory references | Invalid across reboot |
| sexdisplay framebuffer contents | Owned by sexdisplay, not shell policy |
| App process/PD state | Not shell-owned; restored through Track B |
| User document contents | Linen-owned; stored separately |
| Collar grant tokens | Stored separately in Collar service |
| Live capability handles | Cannot persist across reboot; must be re-requested through Collar |

---

## 5. Restore Policy

### Validation sequence (restore must pass all):

1. **Magic check** — `snapshot.magic == 0xAC` else default Scene 0
2. **Version check** — `snapshot.version <= 0x01` else ignore (forward compat: unknown version = discard)
3. **Generation check** — `snapshot.generation >= current_generation` else discard (detect older in-memory candidates within same boot; NOT a durability/security guarantee)
4. **Frame count clamp** — `frame_count = min(frame_count, MAX_FRAMES as u8)`
5. **Per-frame clamp** — `scene_id` clamped, `flags` masked to known bits, geometry clamped to screen bounds
6. **Tab count validation** — if `tab_count > MAX_TABS_PER_FRAME`, reject the snapshot (fail closed). Do not silently clamp corrupt counts.
7. **Active tab validation** — after tab list validates, if `active_tab >= tab_count`, reset to 0 (or None per current shell model) and log `[scene.snapshot.restore.frame] active_tab_reset=1`
8. **Empty frame policy** — frames with `tab_count == 0` are skipped (not restored). Log `[scene.snapshot.restore.skip] reason=empty-frame`. Not fatal to entire snapshot unless `frame_count` claims zero valid frames.
9. **New shell object creation** — restore creates new `ShellFrame`/`ShellTab` objects; does NOT reuse old live object IDs
10. **Minimized + zoomed conflict check** — if both `FRAME_FLAG_MINIMIZED` and `FRAME_FLAG_ZOOMED` are set, minimized wins; zoom flag is cleared during restore. Log proof marker `[scene.snapshot.restore.frame] zoom_cleared_for_minimized=1`.
11. **Rebuild** — call `tile_visible_frames()` after all frames restored
12. **Focus clear** — if `active_scene_id` has no focusable surface, set `FOCUSED_SURFACE_ID = 0`

### Failure policy

| Condition | Behavior |
|-----------|----------|
| All restore entries invalid | Default Scene 0 boot |
| Partial restore (some frames valid) | Restore valid frames, skip invalid |
| Corrupt single frame entry | Skip that frame, continue |
| Missing snapshot (no session data) | No restore, default boot |
| Geometry outside screen bounds | Clamp to screen bounds |
| No focusable surface after restore | Clear focus (safe fallback) |
| Generation stale | Discard snapshot, default boot |
| Empty frame in snapshot | Skip that frame, continue. Log `skipped_empty_frame`. Not fatal. |

### When restore runs

- **V1a (in-memory):** After PD restart in same boot session. Shell checks `SCENE_SNAPSHOT` buffer before defaulting. **Must not claim crash persistence** — same boot session only.
- **V1b (storage-backed):** Blocked until P6 storage design gate approved. At boot after sexstore K/V is available and `boot_load_scene_settings()` completed. Scene restore fires as a second asynchronous load sequence.

---

## 6. Serialization Format

### V1a in-memory struct layout

```rust
/// Shell-local ordering metadata for snapshot freshness.
static mut SNAPSHOT_GENERATION: u8 = 0;

#[repr(C)]
struct SceneSnapshot {
    magic: u8,              // 0xAC (V1a debug marker only — no security/integrity claim)
    version: u8,            // 0x01
    generation: u8,         // shell-local ordering; detects older in-memory candidates within same boot
    active_scene_id: u8,
    frame_count: u8,
    _pad: [u8; 3],         // reserved, zero
    frames: [FrameSnapshot; MAX_FRAMES],
}

#[repr(C)]
struct FrameSnapshot {
    frame_id: u8,
    scene_id: u8,
    flags: u32,
    normal_x: i32,
    normal_y: i32,
    normal_w: u32,
    normal_h: u32,
    active_tab: u8,
    tab_count: u8,
    _pad: [u8; 2],         // alignment
    tabs: [TabSnapshot; MAX_TABS_PER_FRAME as usize],
}

#[repr(C)]
struct TabSnapshot {
    /// Historical/app-hint reference only. NOT a live SurfaceId restore target.
    /// Shell creates new tab objects; this field is for debug/app-identity hints.
    app_hint: u64,
    title_id: u64,
    flags: u32,
    _pad: [u8; 4],         // alignment
}
```

### V1b sexstore layout (reserved, not built until P6 gate)

**Header key `0x02`** (u64):

```
Byte 0: magic       = 0xAC
Byte 1: version     = 0x01
Byte 2: generation
Byte 3: active_scene_id
Byte 4: frame_count
Byte 5-7: reserved  = 0
```

**Frame keys `0x03+N`** (u64 each, multiple keys per frame): design deferred to P6 storage gate.

**V1a does not use sexstore keys.** V1b key design is placeholder only and will be revisited during P6.

---

## 7. Storage Path Decision

### V1a — In-memory snapshot (RECOMMENDED for initial implementation)

```rust
/// Static snapshot buffer for scene/Frame/Tab state.
/// V1a only — in-memory restore within same boot session.
/// V1b extends to sexstore K/V multi-key persistence.
static mut SCENE_SNAPSHOT: Option<SceneSnapshot> = None;
```

Total struct size: 8 + 4*(24 + 8*20) = 8 + 4*184 = 8 + 736 = 744 bytes.

Within static BSS budget for silk-shell (already has larger static buffers).

**V1a touches silk-shell only.** No sexdisplay, kernel, PDX ABI, sexstore, Linen, or package edits. See STOP FIRST conditions for violations.

### V1b — Storage-backed (FUTURE, BLOCKED)

**V1b is blocked until the following are specified:**
1. Corruption handling: what happens on partial write to sexstore? How is partial write detected?
2. Partial-read behavior: sexstore returns u64 per key; how does multi-key atomicity work?
3. Deterministic replay failure modes: what happens when key 0x03 exists but 0x04 does not?
4. Version migration: how does `version > 0x01` get handled during multi-key restore?
5. Key collision: verify no collision with `0x01` (scene appearance) or future allocations

These must be documented in a storage-backed design addendum before any sexstore writes occur.

---

## 8. Implementation Phases

### P0: Audit current shell layout model (no code)
- Inspect `ShellFrame`, `ShellTab`, `FRAMES`, focus/active tab, minimize/zoom state
- Produce mismatch report: what the snapshot spec expects vs what the shell provides
- Check that `ShellFrame.frame_id` allocation is safe for reuse (or document that restore creates new IDs)

### P1: Write FSM/data spec handoff
- Save exact `SceneSnapshot` layout and validation rules in `docs/handoff/SCENE_PERSISTENCE_SPEC_V1.md`
- Include all invariants from this document

### P2: V1a in-memory capture only
- Add static shell-owned `SceneSnapshot` buffer
- Capture current frame/tab layout into buffer
- Log `[scene.snapshot.capture]` with frame count, scene, generation
- No restore yet

### P3: V1a validation only
- Validate snapshot with count/geometry/index/generation rules
- Log `[scene.snapshot.validate.ok]` or `[scene.snapshot.validate.reject]` with reason
- No behavioral changes

### P4: V1a restore shell containers
- Restore shell layout containers only
- Creates new `ShellFrame`/`ShellTab` objects — does NOT reuse old IDs
- No app relaunch, no old SurfaceId resurrection
- Call `tile_visible_frames()` after restore

### P5: Deterministic proof scenarios (V1a)
- Prove: capture → mutate layout → restore layout within same boot
- All 12 proof scenarios below

### P6: V1b storage design gate
- Before any sexstore writes, write a storage-backed design addendum covering corruption, partial writes, versioning, replay, and failure behavior
- Get explicit approval before implementing V1b

### P7: V1b sexstore-backed persistence (future)
- Only after P6 approval
- Implement multi-key sexstore persistence
- Keep V1a buffer as fallback

---

## 9. Object Model

### Core objects

| Object | Type | Owner | Description |
|--------|------|-------|-------------|
| `SceneSnapshot` | struct | shell | Shell-owned in-memory capture of layout intent |
| `SceneSnapshotHeader` | struct | shell | Version, generation (shell-local ordering only), frame_count, magic marker |
| `FrameSnapshot` | struct | shell | Geometry/layout/minimized/zoom state for one frame |
| `TabSnapshot` | struct | shell | Tab title/class/app-hint only — not a live app reference |
| `RestoreIntent` | enum | shell | Request to recreate layout shell containers from snapshot |
| `RestoreResult` | enum | shell | Restored / Skipped / Placeholder / Rejected per frame |
| `SnapshotGeneration` | u8 | shell | Monotonic counter; detect stale snapshots |

### Scene persistence ≠ Session restore

| Concern | Scene persistence (this doc) | Session restore (Track B) |
|---------|-----------------------------|---------------------------|
| What is restored | Shell layout containers (Frames, Tabs, Scene) | Apps, documents, capabilities, services |
| Surface lifecycle | Creates new shell objects | Validates through Track A lifecycle |
| App identity | Not involved | AppIdentity, LaunchManifest |
| Documents | Not involved | Linen DocumentRestoreRef |
| Capabilities | Not involved | Collar CapabilityGrantSnapshot |
| Trust | Not involved | Package signing, Collar verification |
| Storage | In-memory (V1a) or sexstore (V1b) | sexfiles journal + sexstore K/V |

---

## 10. Invariants

1. Snapshot restore creates **new shell objects**, never reuses old live object IDs unless current shell model explicitly supports safe reuse.
2. Snapshot replay must be deterministic — same input always produces same output.
3. Invalid frame count fails closed (default Scene 0).
4. Invalid tab count fails closed or truncates only if explicitly specified.
5. Out-of-bounds geometry is clamped or rejected deterministically (clamp chosen for V1).
6. Minimized/zoomed states must not create impossible layout (conflict check required).
7. Active tab index must be valid or reset to zero.
8. Empty frames are skipped (not fatal to snapshot unless frame_count claims zero valid frames).
9. Snapshot restore cannot grant capabilities.
10. Snapshot restore cannot force focus to a non-focusable surface.
11. Snapshot data is shell-owned; sexdisplay receives only normal bounded render state after shell validation.
12. V1a restore is same-boot only and must not claim crash persistence.
13. V1b restore must tolerate missing/corrupt keys.
14. Any prior SurfaceId appearing in snapshot is historical/debug only — never a live restored target.
15. sexdisplay never reads snapshot data directly; all lifecycle policy stays in shell.
16. A rejected snapshot must leave the current live shell layout unchanged.

---

## 11. STOP FIRST Conditions

- Any kernel edit
- Any PDX ABI edit
- Any sexdisplay policy edit or framebuffer/render path change
- Any app relaunch or session restore behavior added to scene persistence
- Any persistent storage write before V1b corruption/replay spec (P6 gate) is approved
- Any capability handle persistence in snapshot
- Any raw pointer persistence
- Any POSIX path/process/session assumption
- Any broad refactor of shell layout model
- Any sexstore K/V key allocation without documenting in this doc
- Any serialization format exceeding u64 per key without multi-key pattern designed
- Any new heap allocation for serialization buffers
- Any non-shell PD writing scene state
- Any attempt to infer document identity from raw path/app_hint field
- Any attempt to restore a PD or revive a dead protection domain through snapshot
- Any restore path that clears live layout before snapshot validation succeeds

---

## 12. Proof Scenarios

1. Capture one frame / one tab → restore same boot — layout matches.
2. Capture multiple frames / tabs → restore order deterministically.
3. Capture minimized frame → restore minimized state.
4. Capture zoomed frame → restore legal zoom state.
5. Active tab invalid → reset to 0 deterministically.
6. Geometry out of bounds → clamp to screen bounds.
7. Tab count too large → reject snapshot (fail closed).
8. Empty frame encountered → skip that frame, continue restore; log `skipped_empty_frame`.
9. Old SurfaceId present in app_hint field → ignored for live restore.
10. Snapshot generation stale → reject, default Scene 0 (same-boot identification only — not a durability/security guarantee).
11. Corrupt V1b key (future) → fail closed, boot still succeeds.
12. Missing V1b key (future) → no restore, boot still succeeds.

---

## 13. Proof Markers

```
[scene.snapshot.capture] frames=N active_scene=S gen=G
[scene.snapshot.validate.ok] frames=N gen=G
[scene.snapshot.validate.reject] reason=corrupt|version-mismatch|generation-stale|tab-count-overflow|empty-snapshot
[scene.snapshot.restore.start] frames=N gen=G
[scene.snapshot.restore.frame] id=N scene=S flags=M ok=1|skipped|rejected [active_tab_reset=1] [zoom_cleared_for_minimized=1]
[scene.snapshot.restore.skip] reason=no-snapshot|corrupt|version-mismatch|generation-stale|empty-frame|tab-count-overflow
[scene.snapshot.restore.done] restored=N skipped=N rejected=N
```

---

## 14. Implementation Phases (Summary)

| Phase | Name | Description | Depends On |
|-------|------|-------------|------------|
| P0 | Audit | Inspect current shell layout model. No code. | — |
| P1 | Spec handoff | Write `docs/handoff/SCENE_PERSISTENCE_SPEC_V1.md` | P0 |
| P2 | V1a capture | Add `SceneSnapshot` buffer, capture layout. No restore. | P1 |
| P3 | V1a validate | Validate snapshot; log accept/reject. No behavior change. | P2 |
| P4 | V1a restore | Restore shell containers. New objects, no old IDs. | P3 |
| P5 | V1a proof | Run 12 deterministic proof scenarios. | P4 |
| P6 | V1b gate | Write storage-backed design addendum. Get approval. | P5 |
| P7 | V1b sexstore | Implement multi-key sexstore persistence. | P6 |

---

## 15. Cross-Track Dependencies

| Track | Dependency | Phase |
|-------|------------|-------|
| A (COMPOSITOR_LIFECYCLE) | SurfaceId lifecycle guards required before restore validates liveness | A1–A4 must be complete |
| E (PERSISTENT_STORAGE) | sexstore multi-key durability semantics for V1b | E must prove reliability |
| B (APP_LAUNCH_SESSION_RESTORE) | Scene persistence is prerequisite for full session restore | B uses snapshot as starting point |

---

## 16. Future Prompt Names

- `SCENE_PERSISTENCE_AUDIT_V1` — P0 audit
- `SCENE_SNAPSHOT_SPEC_V1` — P1 spec handoff
- `SCENE_SNAPSHOT_CAPTURE_V1A` — P2 capture implementation
- `SCENE_SNAPSHOT_VALIDATE_V1A` — P3 validation implementation
- `SCENE_SNAPSHOT_RESTORE_V1A` — P4 restore implementation
- `SCENE_SNAPSHOT_PROOF_V1A` — P5 deterministic proof
- `SCENE_PERSISTENCE_STORAGE_GATE_V1B` — P6 design gate
- `SCENE_PERSISTENCE_SEXSTORE_V1B` — P7 implementation

---

## 17. Key Design Decisions

### Why V1a (in-memory) before V1b (storage-backed)?
- Zero new dependencies on sexstore multi-key semantics
- Same boot session only — safe because Shell PD restart without full system reboot is the most common recovery path
- Proves the serialization format, validation, and restore logic before storage trust is needed
- V1b requires corruption/replay/versioning spec that does not exist yet (P6 gate)

### Why multi-key vs single large blob for V1b?
- sexstore K/V values are u64 (8 bytes). Frame + tab data exceeds this.
- Multi-key with indexed naming (key `0x03+N`) is the simplest extension of existing pattern.
- Alternative: new sexstore opcode for bulk read/write — STOP FIRST (would require sexstore ABI change).

### Integrity / security (why no XOR claim)
- V1a may use a simple deterministic debug marker (magic byte + generation counter) for basic sanity checking only.
- V1b must **not** claim security or integrity from XOR or magic bytes alone.
- Real integrity requires explicit checksum/MAC/signature design later, likely tied to storage maturity (Track E), package trust (Track G), or Collar verification.
- For V1a, generation metadata may detect older in-memory candidates within same boot — that is sufficient for same-boot use.
