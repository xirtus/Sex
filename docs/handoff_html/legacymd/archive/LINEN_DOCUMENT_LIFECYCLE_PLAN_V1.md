# LINEN_DOCUMENT_LIFECYCLE_PLAN_V1

**Status:** Plan only. No implementation.

**Core principle:** Linen is not a filesystem. Linen is the SexOS document/object/project lifecycle layer above storage. It manages user-facing objects, projects, versions, drafts, tombstones, restore intent, metadata, and app associations without owning low-level persistence, block I/O, or capability authority. Built on E_PERSISTENT_STORAGE_MATURITY guarantees.

---

## 1. Mission

MISSION: F_LINEN_DOCUMENT_LIFECYCLE_PLAN_V1 — Design Linen document/project/object lifecycle on top of E_PERSISTENT_STORAGE_MATURITY. Docs/plan only. No implementation.

---

## 2. Dependency Gates

1. Durable Linen documents are blocked until E9 persistent backend gate passes.
2. No private document names in proof logs until E8 redaction policy exists.
3. No delete/restore UX until E6 tombstone semantics exists.
4. No project/session persistence until E4/E5 schema + corruption behavior exists.
5. LinenObjectId is derived from E key namespace/range model (E2), not from filesystem paths.
6. OpenIntent must use E capability policy (E3) for access validation.
7. Version/checkpoint ordering must use E sequence/schema rules (E4).
8. Linen must not implement its own storage backend — all persistence flows through sexstore/sexfiles.
9. No Linen implementation before E1 audit is complete and E2 key namespace/range model is defined.
10. No Linen app-visible protocol before Collar mediates sensitive grants.

---

## 3. Context

SexOS currently has no document/project lifecycle layer. sexstore (RAM K/V) and sexfiles (PDX VFS) provide storage primitives, but no abstraction for user-facing objects like documents, projects, drafts, versions, or tombstones.

Linen fills this gap by defining:
- What a document is (vs a storage key)
- What a project is (vs a directory)
- How documents move through lifecycle states (new → draft → saved → checkpointed → opened → modified → conflict → tombstoned → restore → deleted)
- How metadata is redacted for privacy
- How apps associate with document types
- How the object model maps onto E storage guarantees

Track E provides the storage trust layer (key ranges, capability checks, schema/versioning, corruption handling, delete/tombstone, redacted proof markers). Linen builds the user-facing lifecycle on top without reimplementing storage.

Current state:
- No Linen object model exists
- No document lifecycle FSM is defined
- No project lifecycle FSM is defined
- No draft/version/checkpoint semantics exist
- No tombstone/restore model exists
- No metadata/redaction model exists
- No app association model exists
- No proof markers for document/project operations exist
- Silk has no document object awareness
- Quil has no document editing lifecycle
- Bell has no document event notification

---

## 4. Why Linen Is Separate from Storage

Storage maturity (E) and document lifecycle (F) must be designed independently because:

1. **Different abstraction levels:** E owns raw key/value ranges, block integrity, and capability-gated persistence. F owns user-facing objects like "my project notes" and "draft version 3."

2. **Different failure modes:** Storage corruption is a byte-level concern. Document corruption is a semantic concern (e.g., draft references a deleted object).

3. **Different timing:** Storage must be deterministic and provably correct before documents can safely reference storage objects. Mixing them means neither is reliable.

4. **Different policy domains:** Storage capability checks whether a PD can write to a key range. Document capability checks whether a user can open a specific version of a project document. These are separate concerns.

5. **Different redaction scope:** Storage redacts key names and values from proof logs. Document redaction must also redact titles, project names, and user-visible labels — a richer semantic model.

6. **Different lifecycle:** Storage objects have a simple lifecycle (valid, tombstoned, corrupt). Documents have a 12-state FSM (New → Draft → Saved → Checkpointed → Opened → Modified → Conflict → Tombstoned → RestorePending → Restored → DeletedFinal → CorruptPlaceholder).

---

## 5. Innovation Goal

Linen should provide a complete document/project lifecycle model where every object transition has a proof marker, every open intent is capability-checked, every tombstone is auditable, every metadata access is redacted by default, and every app association is a hint — not a permission. No raw paths, no POSIX authority, no filesystem semantics, no user-visible storage keys.

---

## 6. Linen Object Model

- **LinenObjectId:** unique identifier for a Linen document/object. Derived from E key namespace, not filesystem path. Not a filename. Opaque to apps.
- **LinenProjectId:** unique identifier for a Linen project. Logically groups LinenObjectIds but is not a directory — membership does not grant authority. Not a folder path.
- **DocumentRef:** reference to a LinenObjectId with optional version/checkpoint hint. Not a file path — must be resolved through capability check. Alone, it grants nothing.
- **ProjectRef:** reference to a LinenProjectId. Membership is metadata, not authority, not a directory listing.
- **ObjectKind:** the type of document (text, spreadsheet, notebook, settings, terminal session, etc.). Defined by app association, not by file extension.
- **ObjectTitle:** user-visible name for a document. Redacted by default in proof logs.
- **MetadataView:** a set of key/value pairs describing document state (title, kind, project membership, version, checkpoint, timestamps). Redacted based on caller capability and E8 policy.
- **RedactedMetadata:** a MetadataView with Private/Secure fields removed. Used in proof logs and cross-PD references.
- **DraftState:** the current draft state of a document (none, unsaved, saved, checkpointed). Not a storage operation.
- **DraftId:** identifier for a specific draft iteration. Used to track uncommitted changes.
- **VersionId:** identifier for a committed version. Ordered using E4 sequence/schema rules.
- **CheckpointId:** identifier for a mid-work snapshot. Lighter weight than a version.
- **ObjectLifecycleState:** current state in the document lifecycle FSM (see §7).
- **ProjectLifecycleState:** current state in the project lifecycle FSM (see §8).
- **TombstoneRecord:** record of a tombstoned document or project. Contains LinenObjectId, tombstone timestamp, proof_sequence_id, and restore eligibility.
- **RestoreIntent:** a request to restore a tombstoned object. Must validate caller capability + tombstone state + revoke/grant state.
- **OpenIntent:** a request to open a document. Contains LinenObjectId, caller identity, requested operation (read/write/admin). Must pass E capability check.
- **AppAssociation:** a hint mapping ObjectKind to a preferred app PD. Not a permission — the target app must have the required capability.
- **CapabilityRequirement:** the set of capabilities required to perform an operation on a Linen object (read, write, delete, restore, admin).
- **LinenProofEvent:** a proof marker for Linen operations. Contains sequence_id, operation, LinenObjectId, redacted metadata, status.
- **LinenPreference:** a user-configurable preference for Linen behavior. Contains preference key, value (validated type), scope (user/shell/Linen-policy), redaction_class.
- **LinenViewPreference:** visual view mode preference. Values: list, grid, cards, compact. UI-only — no lifecycle or authority effect.
- **LinenSortPolicy:** sort order for object lists. Dimensions: name, type, modified, project, custom. Metadata-redacted when sort reveals private fields.
- **LinenGroupPolicy:** grouping policy for objects. Dimensions: project, type, tag, status. No authority change — grouping is display-only.
- **LinenMetadataVisibility:** metadata display level. Values: hidden, redacted, summary, full. Gated by E8 privacy policy — cannot exceed caller's redaction class.
- **LinenThemeToken:** color/icon/accent token from bounded compiled theme set. Cannot claim identity, authority, or trust status.
- **LinenProjectTemplate:** metadata/view preset for new projects. Not a grant container — template cannot grant authority to contained documents.
- **LinenOpenPreference:** default OpenIntent behavior. Values: ask, open last app, preferred app hint. Always capability-checked — preference never bypasses OpenIntent.
- **LinenTombstoneViewPolicy:** tombstone visibility. Values: hidden, show in archive, show in restore view. Cannot make tombstoned objects live.
- **LinenRetentionPolicy:** draft/checkpoint retention duration. Blocked from durable persistence until E gates pass. Memory/proof-only in V1.
- **LinenProofVerbosity:** proof marker detail level. Cannot suppress required safety markers — only controls optional diagnostic fields.
- **LinenPreferenceProofEvent:** a proof marker for preference operations. Contains sequence_id, operation, preference_key, validation_status, redaction_class.

---

## 7. Document Lifecycle FSM

The document lifecycle has 12 states:

1. **New** — LinenObjectId allocated, no content, no storage authority. Exists only in Linen's object registry.
2. **Draft** — Unsaved changes exist in memory. No durable persistence. Proof marker logged.
3. **Saved** — Content committed to storage via E capability-gated write. Durable if E9 gate passed.
4. **Checkpointed** — Mid-work snapshot taken. Lighter than a version. Uses E4 sequence ordering.
5. **Opened** — Document opened by an app via validated OpenIntent. Capability-checked.
6. **Modified** — Opened document has uncommitted changes. Draft state tracks iteration.
7. **Conflict** — Two writers modified the same document. V1: last capability-checked write wins. Conflict state is a notification, not a data merge. Deterministic resolution required: no silent data loss.
8. **Tombstoned** — Document deleted (soft). TombstoneRecord created. Can be restored if restore eligibility passes.
9. **RestorePending** — RestoreIntent submitted, awaiting capability + tombstone + grant validation.
10. **Restored** — Document restored from Tombstoned state. New version created. Old version remains tombstoned.
11. **DeletedFinal** — Terminal state. No restore possible unless E explicitly defines recovery path.
12. **CorruptPlaceholder** — Underlying storage object is corrupt. Linen creates a placeholder. No live document operations.

### State transitions:

**Allowed transitions (with capability checks and proof markers):**

| From | To | Trigger | Capability Check | Proof Marker | Failure Behavior |
|------|----|---------|-----------------|-------------|------------------|
| New | Draft | First edit | None (no storage) | `[linen.object.draft.create]` | Draft not created → New remains |
| Draft | Saved | Commit | E key range + capability policy (E3) | `[linen.object.checkpoint]` | Save rejected → Draft retained |
| Saved | Draft | Edit | OpenIntent must be active | `[linen.object.draft.create]` | Edit rejected → Saved retained |
| Saved | Checkpointed | Snapshot | E4 sequence/schema | `[linen.object.checkpoint]` | Checkpoint rejected → Saved retained |
| Checkpointed | Saved | Resume | None (already validated) | `[linen.object.checkpoint]` | Resume fails → Checkpointed retained |
| Saved | Opened | OpenIntent | Caller identity + operation + key range | `[linen.object.open.allow]` | Capability fail → `[linen.object.open.reject]`, stays Saved |
| Opened | Modified | Edit | OpenIntent was capability-checked | `[linen.object.draft.create]` | Edit blocked → Opened retained |
| Modified | Saved | Commit | Re-validates capability | `[linen.object.checkpoint]` | Rejected → Modified retained |
| Modified | Conflict | Concurrent write | Last write validated | `[linen.error]` + conflict | Last capability-checked write wins |
| Conflict | Saved | Resolve | Manual or policy resolution | `[linen.object.checkpoint]` | Unresolved → Conflict persists |
| Opened | Tombstoned | Delete | Caller + delete capability | `[linen.object.tombstone]` | Rejected → Opened retained |
| Saved | Tombstoned | Delete | Caller + delete capability | `[linen.object.tombstone]` | Rejected → Saved retained |
| Draft | Tombstoned | Discard | Caller + delete capability | `[linen.object.tombstone]` | Rejected → Draft retained |
| Tombstoned | RestorePending | Restore intent | RestoreIntent submitted | `[linen.object.restore.intent]` | Rejected → Tombstoned retained |
| RestorePending | Restored | Validate | Capability + tombstone + revoke state | `[linen.object.restore.allow]` | Validation fail → `[linen.object.restore.reject]`, stays Tombstoned |
| RestorePending | Tombstoned | Reject | Capability or revoke check failed | `[linen.object.restore.reject]` | N/A (rejection IS the terminal action) |
| Restored | Draft | Edit restored | Re-validates capability | `[linen.object.draft.create]` | Rejected → Restored retained |
| Tombstoned | DeletedFinal | Permanent delete | E StoreAdmin capability | `[linen.object.tombstone]` (DeletedFinal) | Rejected → Tombstoned retained |
| Any | CorruptPlaceholder | Storage corruption | E5 corruption detection | `[linen.object.corrupt.placeholder]` | Corruption undetected → data risk (E5 invariant) |
| CorruptPlaceholder | New | Create replacement | None (new object) | `[linen.object.new]` | Replacement fails → placeholder remains |

**Forbidden transitions (never allowed):**

| From | To | Why Forbidden |
|------|----|---------------|
| New | Tombstoned | Object has no content or authority to delete |
| New | Opened | Must enter Draft or Saved first |
| Draft | Opened | Must save before open (save→open path only) |
| Draft | Checkpointed | Checkpoints are from Saved state only |
| Tombstoned | Opened | Cannot open tombstoned object as live — must restore first |
| Tombstoned | Draft | Same as above — must restore first |
| DeletedFinal | Any | Terminal state — no transitions out |
| RestorePending | Opened | Must complete restore cycle first |
| CorruptPlaceholder | Opened | No live operations on corrupt data |
| CorruptPlaceholder | Draft | No draft from corrupt data |
| CorruptPlaceholder | Saved | No save from corrupt data |
| Conflict | Tombstoned | Cannot delete a conflicted document — resolve first |
| Any | DeletedFinal (direct) | Must go through Tombstoned first (permanent delete requires soft delete step) |

---

## 8. Project Lifecycle FSM

The project lifecycle has 7 states:

1. **NewProject** — LinenProjectId allocated. No members.
2. **ActiveProject** — Project has members and is in use.
3. **ArchivedProject** — Project visible but not active. Members not automatically accessible.
4. **TombstonedProject** — Project deleted (soft). All members tombstoned or detached.
5. **RestorePendingProject** — RestoreIntent for project submitted, awaiting validation.
6. **RestoredProject** — Project restored. Members re-associated but each member's capability validated individually.
7. **CorruptProjectPlaceholder** — Project metadata corrupt. Placeholder created.

### Project state transitions:

| From | To | Trigger | Capability Check | Proof Marker |
|------|----|---------|-----------------|-------------|
| NewProject | ActiveProject | Add first member | Project create capability | `[linen.project.new]` |
| ActiveProject | ArchivedProject | Archive action | Project admin capability | `[linen.project.archive]` |
| ActiveProject | TombstonedProject | Delete project | Caller + delete capability | `[linen.object.tombstone]` |
| ArchivedProject | ActiveProject | Unarchive | Project admin capability | `[linen.project.new]` |
| ArchivedProject | TombstonedProject | Delete project | Caller + delete capability | `[linen.object.tombstone]` |
| TombstonedProject | RestorePendingProject | Restore intent | RestoreIntent submitted | `[linen.object.restore.intent]` |
| RestorePendingProject | RestoredProject | Validated | Capability + tombstone + revoke state | `[linen.object.restore.allow]` |
| RestorePendingProject | TombstonedProject | Rejected | Validation failed | `[linen.object.restore.reject]` |
| RestoredProject | ActiveProject | Resume | Re-validates membership | `[linen.project.new]` |
| Any | CorruptProjectPlaceholder | Metadata corruption | E5 corruption detection | `[linen.object.corrupt.placeholder]` |

### Forbidden project transitions:

| From | To | Why Forbidden |
|------|----|---------------|
| NewProject | TombstonedProject | Must have members first or explicit empty-project delete |
| NewProject | ArchivedProject | Cannot archive what was never active |
| TombstonedProject | ActiveProject | Must restore first |
| TombstonedProject | ArchivedProject | Cannot archive a tombstoned project |
| CorruptProjectPlaceholder | Any | No operations on corrupt metadata |
| ArchivedProject | RestorePendingProject | Archive is not tombstone — use unarchive, not restore |

### Key rules:
- Project membership does not grant document access. Each document's OpenIntent is checked individually.
- Archiving a project does not tombstone its documents — they remain accessible individually.
- Restoring a project does not automatically restore all members — each member's restore eligibility is checked.
- CorruptProjectPlaceholder is a metadata corruption, not a storage corruption. If underlying storage is corrupt, individual documents become CorruptPlaceholder.
- Project delete (→ TombstonedProject) does NOT delete member documents — each member must be tombstoned individually through E6.

---

## 9. Draft/Version/Checkpoint Model

### Drafts:
- A Draft is an uncommitted change set in memory.
- DraftId tracks iteration within a session.
- No durability guarantee until Saved.
- Proof marker `[linen.object.draft.create]` logged on each draft.
- Drafts cannot bypass E capability/key-range checks on save.

### Versions:
- A Version is a committed, immutable snapshot of document content.
- VersionId is ordered using E4 sequence/schema rules.
- Versions are capability-checked at read time — version history is not automatically accessible.
- Version creation is a storage write through E capability-gated path.

### Checkpoints:
- A Checkpoint is a mid-work snapshot, lighter than a full version.
- CheckpointId is local to a session until promoted to a Version.
- Checkpoints before E4/E9 are memory-only or proof-only.

### Constraints:
- Version ordering is monotonic — no reordering or insertion.
- Checkpoint promotion to Version requires E capability check.
- Draft save cannot create a Version without going through Saved state.
- V1: no automatic versioning — user-initiated or app-initiated only.

---

## 10. Tombstone/Delete/Restore Model

### Delete:
- Soft delete transitions document to Tombstoned state. This is not POSIX unlink — delete is a state transition, not filesystem removal. The underlying E6 tombstone handles storage-level semantics.
- TombstoneRecord contains: LinenObjectId, timestamp, proof_sequence_id, caller identity, restore eligibility flag.
- Tombstoned document cannot be opened as live without restore validation.
- E6 provides underlying storage tombstone semantics; F6 defines the user-facing delete/restore UX. Linen delete is not a storage erase — it is a lifecycle state change.

### Restore:
- RestoreIntent submitted by caller with capability validation.
- Restore validates: caller identity, capability grant, tombstone state, revoke state.
- If grants were revoked between delete and restore, restore is rejected — `[linen.object.restore.reject]` with reason.
- Restored document gets a new VersionId. Old version remains tombstoned.

### Permanent delete:
- DeletedFinal is terminal. No restore path unless E explicitly defines recovery.
- Only E StoreAdmin capability can issue permanent delete.

### Key rules:
- RestoreIntent must not resurrect revoked grants.
- Tombstoned object cannot be opened as live without validate→restore sequence.
- DeletedFinal is truly final — proof marker, no undo.

---

## 11. Metadata/Redaction Model

### Metadata fields:
- ObjectTitle (user-visible name)
- ObjectKind (document type)
- Project membership (LinenProjectId list)
- VersionId / CheckpointId
- Timestamps (created, modified, tombstoned, restored)
- AppAssociation hint
- CapabilityRequirement summary

### Redaction rules:
- ObjectTitle is Private by default — redacted in proof logs.
- ObjectKind is Session — logged, but full details redacted.
- Project membership is Session — membership logged, member IDs redacted.
- Version/checkpoint IDs are Public — needed for ordering.
- Timestamps are Session by default — may be elevated to Public for audit.
- AppAssociation hint is Session — key logged, details redacted.
- CapabilityRequirement summary is Secure — operation type only in logs.

### Proof log redaction:
- Proof markers use RedactedMetadata — Private/Secure fields removed.
- `[linen.metadata.redact]` logged when redaction is applied.
- Redaction policy is E8-compatible — same classes (Public/Session/Private/Secure).
- Private titles/names are never persisted in public proof logs.

---

## 12. App Association/Open-With Model

### AppAssociation:
- A hint mapping ObjectKind to a preferred app PD identifier.
- Not a permission — the target app must hold the required capability.
- Stored as metadata on the Linen object, not in a global registry.
- Multiple apps can associate with the same ObjectKind — user preference selects default.

### OpenIntent flow:
1. Caller (Silk, Quil, or other PD) submits OpenIntent with LinenObjectId + requested operation.
2. F validates caller capability against CapabilityRequirement FIRST — capability check always precedes app association lookup.
3. If caller lacks capability → `[linen.object.open.reject]` with reason. App association is never consulted.
4. If caller has capability → F checks AppAssociation hint to determine target app.
5. If target app lacks required grant → `[linen.object.open.reject]` — app association hint is not authority.
6. If both caller capability and target app grant pass → `[linen.object.open.allow]`.

### Key rules:
- OpenIntent is capability-checked — LinenObjectId alone grants nothing.
- App association hints are advisory — F does not enforce app-to-object binding.
- If no app association exists, OpenIntent may suggest available apps but cannot force open.
- App association is a hint, not permission — Collar mediates sensitive grants.

---

## 13. Silk/Bell/Quil/Collar/Mesh Integration

### Silk:
- Silk may display Linen objects (document cards, project lists, recent documents).
- Silk does not own document lifecycle — state transitions are F-managed.
- Silk displays state derived from MetadataView, not from raw storage.
- Silk uses redacted metadata by default — no private document names in chrome.

### Bell:
- Bell may notify about Linen events (document shared, project archived, restore available).
- Notifications carry RedactedMetadata — no private titles/content.
- Bell does not own document lifecycle or storage policy.

### Quil:
- Quil may edit Linen documents as an app PD.
- Quil edit intent goes through E capability check — Quil does not bypass F lifecycle.
- Quil may view version history through F's version/checkpoint interface, not raw storage.

### Collar:
- Collar mediates sensitive grants for Linen operations (open, delete, restore, admin).
- No app-visible Linen protocol before Collar mediates grants.
- OpenIntent without Collar grant is rejected.

### Mesh:
- Mesh may visualize Linen object/capability relationships (object graph, project membership, restore eligibility).
- Mesh visualizes relationships but grants nothing — no authority from graph display.
- Mesh uses RedactedMetadata for display.

### sexdisplay:
- sexdisplay never handles Linen semantics — pixels only.
- Linen object cards, project lists, and document views are rendered by Silk/Quil through sexdisplay's existing pixel path.

---

## 14. Storage Boundary with E

### Linen does NOT own:
- Raw key range authority — E storage policy owns all key range decisions.
- Block/file persistence — sexfiles/sexshop/store layer owns all storage I/O.
- Capability enforcement for storage operations — E capability policy (E3) owns this.
- Schema/versioning of stored data — E sequence/schema/version model (E4) owns this.
- Corruption detection and recovery — E corruption handling (E5) owns this.
- Storage-level tombstone semantics — E delete/tombstone (E6) owns this.
- Proof marker format for storage operations — E deterministic proofs (E7) owns this.
- Privacy redaction for storage proof logs — E redaction policy (E8) owns this.
- Persistent backend decisions — E persistent backend gate (E9) owns this.

### Linen owns:
- Document object model (LinenObjectId, DocumentRef, ObjectKind, ObjectTitle)
- Project object model (LinenProjectId, ProjectRef, membership)
- Draft lifecycle (DraftId, DraftState, iteration tracking)
- Version/checkpoint lifecycle (VersionId, CheckpointId, ordering)
- Metadata view model (MetadataView, RedactedMetadata)
- Restore intent validation (RestoreIntent, tombstone eligibility)
- User-facing document/project actions (open, close, delete, restore, archive)
- App association hints (AppAssociation, ObjectKind→app mapping)
- Object references for Silk/Bell/Quil/Collar/Mesh/Harp/etc.

### E dependency mapping

| Linen Operation | Requires E Gate | What E Provides | Blocked Until |
|----------------|----------------|-----------------|---------------|
| Create draft (New → Draft) | E2 key namespace/range + E3 capability policy | ObjectId derivation from key range; caller capability check | E2 + E3 complete |
| Save draft (Draft → Saved) | E4 sequence/schema model | VersionId ordering, monotonic sequence | E4 complete |
| Checkpoint (Saved → Checkpointed) | E4 sequence/schema + E5 corruption behavior | Sequence ordering; storage integrity guarantee | E4 + E5 complete |
| Open document (any state → Opened) | E3 capability policy | Caller identity + key range + operation validation | E3 complete |
| Delete/tombstone (→ Tombstoned) | E6 delete/tombstone semantics | Storage-level tombstone, idempotent delete | E6 complete |
| Restore (→ Restored) | E6 tombstone + E3 capability policy + revoke state | Tombstone eligibility; grant validation; revoke check | E6 + E3 complete |
| Metadata log (any proof marker) | E8 privacy/redaction policy | Redaction classes, Private/Secure suppression | E8 complete |
| Durable persistence (Saved survives power cycle) | E9 persistent backend gate | Storage durability guarantee | E9 complete (V1: RAM-only) |
| sexfiles/sexshop integration | E10 sexfiles/sexshop integration | VFS/block-backed paths, handover patterns | E10 complete |
| Version/checkpoint ordering | E4 sequence/schema model | VersionId monotonic ordering | E4 complete |
| Conflict resolution | E5 corruption/partial-failure | Write validation, partial failure detection | E5 complete |

### Reference model:
- Linen references storage objects by capability-scoped refs, not raw paths.
- Linen never constructs raw storage keys — all key derivation goes through E key namespace.
- Document persistence flows: Linen → OpenIntent → E capability check → sexstore/sexfiles write.
- Tombstone flow: Linen delete → TombstoneRecord → E6 storage tombstone.
- Proof flow: Linen operation → LinenProofEvent → (cross-reference with StorageProofEvent).

---

## 15. Invariants

**Authority & Identity (1-3, 13, 20, 22):** LinenObjectId/DocumentRef are not storage authority — capability check required. OpenIntent must be capability-checked before any op. App association is hint, not permission — target app must hold capability. No raw cross-PD document pointers. LinenObjectId unique per document (E2 collision-free allocation).

**Restore & Tombstone (4-6, 21, 27, 31, 41-42):** RestoreIntent must not resurrect revoked grants. Tombstoned objects need restore validation before live open. DeletedFinal is terminal unless E defines recovery. Delete is a state transition, not storage erase — no POSIX unlink. Normal users cannot reach DeletedFinal (StoreAdmin only). Restore rejection includes structured reason. Tombstone visibility requires RestoreIntent validation. Restore cannot bypass revoke/capability checks.

**Corruption & Safety (7, 11, 25, 43):** Corrupt objects → placeholders, never live. Users never see CorruptPlaceholder — silently "unavailable". Version/checkpoint ordering uses E4 rules — no out-of-order. Draft/checkpoint retention respects E sequence/schema rules.

**Redaction & Privacy (8-9, 28, 37-38):** Metadata proof logs redacted by default — Private/Secure suppressed. Private titles/names never in public proof logs. All cross-PD metadata uses RedactedMetadata — raw MetadataView never leaves Linen. Preference logs exclude private data. Metadata display cannot override E8 redaction.

**Storage Boundaries (10, 14-19, 35-36):** Draft save cannot bypass E capability/key-range checks. Silk displays but does not own lifecycle. Bell notifies but does not own lifecycle/storage policy. Quil edits through capability check, not bypass. Collar mediates sensitive grants. Mesh visualizes but grants nothing. sexdisplay is pixels-only. Preferences memory/proof-only until E gates — Linen does not own preference storage.

**Lifecycle & UX (23-24, 26, 29-30, 32):** No document outside 12-state FSM — every transition has defined outcome. UX exposes ≤3 states (Draft/Saved/Deleted). OpenIntent caching session-scoped, explicitly revocable. Open latency bounded by PDX round-trip — no storage I/O. Developer API: ≤5 core ops (create/open/save/delete/restore). Mesh: Active/Tombstoned objects, Active/Archived/Tombstoned projects.

**Customization Boundaries (33-34, 39-40, 44-48):** Preferences user/shell/Linen-policy owned, not app-owned. Bad values reject or clamp deterministically. App association remains hint — OpenIntent validates regardless. Templates cannot grant authority — metadata only. Proof verbosity cannot suppress required safety markers. View/sort/group preferences display-only — no lifecycle mutation. Color/icon/theme tokens bounded to compiled set — no authority claim. Reset-to-safe-default restores compiled defaults. Accessibility alternatives required for all visual-only customizations.

---

## 16. STOP FIRST Conditions

- Any proposal using POSIX path authority for Linen objects
- Any filesystem rewrite or directory abstraction
- Any disk persistence before E9 persistent backend gate passes
- Any private metadata logging before E8 redaction policy exists
- Any delete/restore UX before E6 tombstone semantics exists
- Any document versioning before E4 sequence/schema model exists
- Any app open-with that bypasses Collar/E capability policy
- Any Linen grant of authority because an object is in a project
- Any raw pointers/shared buffers/backing buffer redesign
- Any sexdisplay document semantics
- Any shell focus/layout changes for Linen objects
- Any package trust changes
- Any crash log viewer implementation
- Any kernel/ABI/sex-pdx edit
- Any broad refactor not scoped to a single F phase
- Any app format/editor implementation
- Any implementation before F1 audit is complete
- Any POSIX unlink semantics for Linen delete — delete is a lifecycle state transition, not filesystem removal
- Any customization using raw POSIX path authority
- Any user-editable code/plugins/scripts/macros in customization
- Any untrusted theme packs with executable behavior
- Any app-owned document lifecycle policy via customization
- Any app-owned security/privacy policy via customization
- Any project membership-as-permission via customization
- Any metadata display customization that bypasses E8 redaction
- Any preference persistence before E storage gates
- Any restore customization that bypasses revoked grant rejection
- Any DeletedFinal recovery without explicit E policy
- Any key namespace/range changes from Linen preferences
- Any disabling of required proof markers via verbosity preference
- Any private metadata in preference logs
- Any raw color/layout values without bounds checking
- Any visual-only customization with no accessibility alternative
- Any Linen owning renderer/shell/storage policy via preferences

---

## 17. Proof Scenarios

### Proof markers

```
[linen.audit.start]
[linen.object.new]
[linen.object.open.intent]
[linen.object.open.allow]
[linen.object.open.reject]
[linen.object.draft.create]
[linen.object.checkpoint]
[linen.object.tombstone]
[linen.object.restore.intent]
[linen.object.restore.allow]
[linen.object.restore.reject]
[linen.object.corrupt.placeholder]
[linen.project.new]
[linen.project.archive]
[linen.metadata.redact]
[linen.app.association.hint]
[linen.error]
[linen.pref.load]
[linen.pref.validate.ok]
[linen.pref.validate.reject]
[linen.pref.apply]
[linen.pref.reset]
[linen.pref.persist.reject]
[linen.pref.redact]
[linen.pref.accessibility.warn]
```

### Scenarios (condensed)

**Core (1-20):** New→`[linen.object.new]`. OpenIntent(valid)→`[linen.object.open.allow]`. OpenIntent(no cap/no grant)→`[linen.object.open.reject]`. Draft→`[linen.object.draft.create]`. Checkpoint before E4/E9→STOP FIRST. Tombstone live→reject `[linen.object.open.reject]`. RestoreIntent→`[linen.object.restore.allow]` or `.reject`. Revoked grant blocks restore→`.reject` with reason. DeletedFinal→no restore `[linen.object.tombstone]`. Corrupt→`[linen.object.corrupt.placeholder]`. Private title redacted→`[linen.metadata.redact]`. Project membership grants no child access. Archive→`[linen.project.archive]`. Bell→RedactedMetadata. Quil edit→OpenIntent flow. Mesh grants nothing. Silk→MetadataView. Missing backend→CorruptPlaceholder. No raw paths→OpenIntent requires LinenObjectId.

**Negative (21-28):** Project membership→OpenIntent per-doc. AppAssoc without grant→`.reject`. Private title in log→`.metadata.redact`. Corrupt as live→reject. Restore after revoke→`.restore.reject`. DeletedFinal restore→reject. Raw path→reject. Persist before E9→STOP FIRST.

**Customization (29-44):** Valid pref→`[linen.pref.load]`→`.validate.ok`→`.apply`. Invalid→`.validate.reject`, clamped. Sort/group→UI only `.apply`, no cap check. Template→metadata only `.apply`. AppAssoc hint→`.apply` but OpenIntent still validates. Metadata=full but E8 blocks→`.metadata.redact`. Tombstone visible but not live→`.open.reject`. Restore pref cannot bypass revoke→`.restore.reject`. Retention before E gates→`.pref.persist.reject`. Persist before E gates→`.pref.persist.reject`. Min proof verbosity→safety markers still fire. Token identity claim→`.pref.validate.reject`. Reset→`.pref.reset`. Missing accessibility→`.pref.accessibility.warn`. Raw path→`.pref.validate.reject`. Plugin/script→STOP FIRST.

---

## 18. Minimal Phase Ladder

1. **F1_LINEN_AUDIT_V1** — Audit current document/project lifecycle gaps: what exists (nothing), what E guarantees are available, what integration points exist (Silk, Bell, Quil, Mesh, Collar, sexdisplay). Document the gap. No code.

2. **F2_LINEN_OBJECT_MODEL_SPEC_V1** — Define LinenObjectId, LinenProjectId, DocumentRef, ProjectRef, ObjectKind, ObjectTitle, MetadataView, RedactedMetadata. Handoff doc.

3. **F3_DOCUMENT_LIFECYCLE_FSM_V1** — Define 12-state document lifecycle FSM: state definitions, transitions, guards, proof markers per transition. Handoff doc.

4. **F4_PROJECT_LIFECYCLE_FSM_V1** — Define 7-state project lifecycle FSM: state definitions, transitions, membership rules (no automatic grant), archiving semantics. Handoff doc.

5. **F5_DRAFT_VERSION_CHECKPOINT_MODEL_V1** — Define DraftId, VersionId, CheckpointId, ordering rules (E4-compatible), save/commit/promote semantics, memory-only vs durable distinction. Handoff doc.

6. **F6_TOMBSTONE_RESTORE_MODEL_V1** — Define TombstoneRecord format, RestoreIntent validation (capability + tombstone state + revoke state), DeletedFinal terminal semantics, E6 integration. Handoff doc.

7. **F7_METADATA_REDACTION_MODEL_V1** — Define MetadataView fields, redaction classes per field, RedactedMetadata format, proof log redaction, E8-compatible policy. Handoff doc.

8. **F8_OPEN_INTENT_APP_ASSOCIATION_V1** — Define OpenIntent flow, AppAssociation hints, capability check integration (caller identity + operation), Collar grant mediation. Handoff doc.

9. **F9_LINEN_INTEGRATION_BOUNDARIES_V1** — Define integration boundaries with Silk (display, not lifecycle), Bell (notification, not policy), Quil (edit, not bypass), Mesh (visualize, not grant), Collar (grants, not hints), sexdisplay (pixels only). Handoff doc.

10. **F10_LINEN_PROOF_SCENARIOS_V1** — Define all 20 proof scenarios, proof marker format, redaction integration, cross-reference with E StorageProofEvent. Handoff doc.

---

## 19. Handoff Files

- `docs/handoff/F_LINEN_DOCUMENT_LIFECYCLE_PLAN_V1.md` — this document (overview)
- `docs/handoff/LINEN_OBJECT_MODEL_V1.md` — LinenObjectId, DocumentRef, ObjectKind, MetadataView (F2)
- `docs/handoff/LINEN_DOCUMENT_LIFECYCLE_FSM_V1.md` — 12-state FSM, transitions, guards (F3)
- `docs/handoff/LINEN_PROJECT_LIFECYCLE_FSM_V1.md` — 7-state FSM, membership rules (F4)
- `docs/handoff/LINEN_DRAFT_VERSION_CHECKPOINT_V1.md` — DraftId, VersionId, ordering (F5)
- `docs/handoff/LINEN_TOMBSTONE_RESTORE_V1.md` — TombstoneRecord, RestoreIntent, E6 integration (F6)
- `docs/handoff/LINEN_METADATA_REDACTION_V1.md` — MetadataView, redaction classes, E8 policy (F7)
- `docs/handoff/LINEN_OPEN_INTENT_APP_ASSOCIATION_V1.md` — OpenIntent, AppAssociation, capability checks (F8)
- `docs/handoff/LINEN_INTEGRATION_BOUNDARIES_V1.md` — Silk/Bell/Quil/Collar/Mesh/sexdisplay boundaries (F9)
- `docs/handoff/LINEN_PROOF_SCENARIOS_V1.md` — 20 proof scenarios, proof marker format (F10)

---

## 20. Future Sub-Prompt Names

- `F1_LINEN_AUDIT_V1`
- `F2_LINEN_OBJECT_MODEL_SPEC_V1`
- `F3_DOCUMENT_LIFECYCLE_FSM_V1`
- `F4_PROJECT_LIFECYCLE_FSM_V1`
- `F5_DRAFT_VERSION_CHECKPOINT_MODEL_V1`
- `F6_TOMBSTONE_RESTORE_MODEL_V1`
- `F7_METADATA_REDACTION_MODEL_V1`
- `F8_OPEN_INTENT_APP_ASSOCIATION_V1`
- `F9_LINEN_INTEGRATION_BOUNDARIES_V1`
- `F10_LINEN_PROOF_SCENARIOS_V1`

---

## 21. Premortem Analysis

**Premise:** Assume this plan failed 6 months after acceptance. Below are the identified failure modes, their categories, and the revised safest path hardening applied above.

### Failure Mode Table

| # | Failure Mode | Category | Severity | Hardening Applied |
|---|-------------|----------|----------|-------------------|
| 1 | **F bypasses E gates** — Linen implemented before E2 key ranges, E3 capability policy, E4 schema/version, E6 delete/tombstone, E8 privacy redaction pass → document lifecycle built on unstable storage | Dependency stall (§2) | **Critical** | §2 gates 1-10 lock F behind E maturity gates; STOP FIRST §16 prohibits implementation before F1 audit |
| 2 | **Linen becomes a filesystem** — DocumentRef treated as file path, LinenObjectId derived from directory structure → POSIX semantics creep | Scope creep (§4) | **Critical** | Core principle: "Linen is not a filesystem"; §14 storage boundary defines what Linen does not own; STOP FIRST for POSIX path authority; §6 clarifies DocumentRef is not a file path |
| 3 | **Project membership grants automatic access** — Membership seen as authority, child documents opened without individual capability checks | Invariant violation (§15.12) | **Critical** | §15.12 invariant; §8 project lifecycle explicitly states "membership does not grant document access"; §17 scenario 13; negative test 21 |
| 4 | **Linen tries to own storage backend** — F implements its own sexstore/sexfiles alternative instead of using E layer | Scope creep (§14) | **Critical** | §14 boundary: Linen does not own block/file persistence; §2.8: Linen must not implement its own storage backend |
| 5 | **Private titles leaked in proof logs** — Metadata proof markers log ObjectTitle without redaction → privacy leak | Privacy leak (§15.8, §15.9) | **High** | §15.8/15.9 invariants require default redaction; §11 redaction model specifies Private class for ObjectTitle; negative test 23 |
| 6 | **App association becomes permission system** — AppAssociation hint treated as authority, apps opened without Collar grant | Scope creep (§12) | **High** | §12: "App association is a hint, not permission"; §15.13 invariant; §17 scenario 4; negative test 22 |
| 7 | **Restore resurrects revoked grants** — RestoreIntent ignores revoke state, restored document grants access that should not exist | Invariant violation (§15.4) | **High** | §15.4 invariant; §10: "RestoreIntent must not resurrect revoked grants"; §17 scenario 9; negative test 25 |
| 8 | **Durable persistence assumed before E9** — RAM drafts treated as durable | Dependency stall (§2.1) | **High** | §2.1 blocks durable until E9; drafts memory/proof-only |
| 9 | **Corrupt object becomes live document** — Corrupt data opened bypassing CorruptPlaceholder | Invariant violation (§15.7) | **High** | §15.7: corrupt → placeholders; E5 detection; negative test 24 |
| 10 | **PD boundary drift** — Silk owns lifecycle or sexdisplay renders semantics → lifecycle inconsistent across PDs | Renderer ownership (§15.14, §15.19) | **High** | §15.14 (Silk: display not lifecycle) + §15.19 (sexdisplay: pixels only). STOP FIRST for shell focus/layout or sexdisplay semantics |
| 11 | **F1 audit skipped** — F2-F10 designed without understanding current gaps | Dependency stall (§2.9) | **High** | §2.9: no implementation before F1 audit; STOP FIRST for pre-audit implementation |
| 12 | **Tombstone/delete becomes POSIX unlink** — Storage erase, restore impossible | Invariant violation (§15.21) | **High** | §15.21: delete is state transition, not storage erase; STOP FIRST for POSIX unlink |
| 13 | **Quil edit path bypasses OpenIntent** — Direct storage write without capability check | MPK/PDX fault (§15.16) | **High** | §15.16: Quil must go through capability check |
| 14 | **DeletedFinal restored by convenience UX** — Undo resurrects terminal state | Scope creep (§15.6) | **High** | §15.6: DeletedFinal is terminal; no restore path |
| 15 | **Project archive orphans active docs** — Documents invisible but not tombstoned | Process failure (§8) | **Moderate** | §8: archive does not tombstone documents; member docs remain accessible individually |
| 17 | **Linen object ID reuse** — Collision across documents | Invariant violation | **Moderate** | §6: E2 key namespace collision-free; no reuse in V1 |
| 18 | **Conflict no resolution** — Document stuck in Conflict | Process failure | **Moderate** | §7: last capability-checked write wins; conflict→Saved transition |
| 19 | **Missing backend panic** — sexstore unavailable → PD crash | MPK/PDX fault | **Moderate** | §15.7: corrupt→CorruptPlaceholder, not panic |
| 20 | **Version ordering drift** — Out-of-order versions | Invariant violation (§15.11) | **Moderate** | §15.11: E4 sequence rules |
| 21 | **sexdisplay renders semantics** — Violates pixels-only | Renderer ownership (§15.19) | **Moderate** | §15.19: sexdisplay never handles Linen semantics |
| 22 | **Mesh graph grants authority** — Visualization implies grant | MPK/PDX fault (§15.18) | **Moderate** | §15.18: Mesh visualizes but grants nothing |

### Revised Safest Path (condensed)

1. **F1 audit mandatory** — No F implementation before audit. Documents lifecycle gaps, E guarantees, integration points.
2. **Flat object model first** — F2: LinenObjectId/Ref/Kind/MetadataView. No nesting. Hierarchy after lifecycle proofs pass.
3. **OpenIntent before app association** — Capability check always precedes app association. AppAssociation is display hint only.
4. **Redaction before public proof logs** — E8 before persistent Linen proof markers. Private titles never in logs.
5. **Tombstone before restore** — E6 delete semantics before any restore UX.
6. **Draft/checkpoint after E4** — VersionId ordering depends on E4 monotonic sequence. Before E4: memory/proof-only.
7. **Durable persistence after E9** — V1 RAM-only. No power-cycle survival before E9.
8. **sexfiles/sexshop after E10** — Linen does not implement own storage backend.
9. **No editor/file-browser UX before F3+F4+F10** — Lifecycle proofs must precede editing UI.

---

## 22. Exceeded Hypothesis Analysis

**Premise:** Another OS/object workspace beat Linen across 10 dimensions. Each row maps the rival advantage, the loss mode, and the SexOS-native hardening (referencing existing §15 invariants).

| Rival Advantage | SexOS-Native Fix | Invariant/Proof Gate |
|----------------|------------------|---------------------|
| **Reliability** — atomic saves, no undefined states | Every FSM transition has defined outcome. No undefined states. | §15.23 | F3+E5 |
| **Simplicity** — open-save-close UX | FSM internal. UX shows ≤3 states (Draft/Saved/Deleted). | §15.24 | F3+F9 |
| **Document safety** — no corrupt docs surfaced | CorruptPlaceholder → "unavailable". Draft auto-recover in memory. | §15.25 | F5+F7 |
| **Project speed** — fast member switching | Session-scoped OpenIntent caching. First open validates, subsequent cached. Revocable. | §15.26 | F8+E3+Collar |
| **Restore clarity** — one-click, handles revoked grants | RestoreIntent returns structured result (missing caps, revoked grants, tombstone state). | §15.27 | F6 |
| **Privacy** — no metadata leaks | All cross-PD metadata → RedactedMetadata. Raw MetadataView never leaves Linen. | §15.28 | F7+E8 |
| **Speed** — instant operations | Latency bounded by PDX round-trip. No storage I/O in open path. | §15.29 | F8+E3 |
| **Dev workflow** — 5-operation API | Developer contract: create/open/save/delete/restore. Edge cases opt-in. | §15.30 | F10 |
| **User trust** — no permanent data loss | Normal users never reach DeletedFinal. StoreAdmin only. | §15.31 | F6+E3 |
| **Visual clarity** — Mesh shows clean graph | Mesh shows Active/Tombstoned objects, Active/Archived/Tombstoned projects. | §15.32 | F9 |

### SexOS-Native Patterns (Not App Clones)

1. **Object history timeline** → VersionId ordering (E4) + flat version list with redacted titles. Mesh displays as timeline graph. No diff/content history.
2. **Project graph clarity** → LinenProjectId membership is metadata-only. Mesh shows capability-scoped edges without granting access.
3. **Safe restore UX** → RestoreIntent returns structured eligibility: "X caps required, Y grants revoked — restore will skip." User confirms.
4. **Privacy labels** → E8 classes on each MetadataView field. Silk/Bell/Mesh/Quil receive RedactedMetadata only. Private titles redacted at source.
5. **Command-palette actions** → Enumerate available operations per LinenObjectId via capability grant. Only permitted ops shown.
6. **Deterministic audit trail** → Every FSM transition produces proof marker. Sequence forms audit trail: new→draft→checkpoint→tombstone→restore.
7. **Visual capability graph (Mesh)** → Directed graph: object→app→grant. Read-only. Never modifies grants.
8. **Explicit grant UX (Collar)** → Rejected OpenIntent returns missing capability. Collar surfaces grant request. V1 rejects with reason; future adds grant-request flow.

### New STOP FIRST from Exceeded Analysis (also in §16)

UX exposing internal FSM states; cross-PD metadata before E8; OpenIntent caching without revoke mechanism; DeletedFinal for non-StoreAdmin; editor/file-browser UX before F3+F4+F10 pass.

### New Invariants from Exceeded Analysis (already enumerated in §15.23-32)

§15.23-32 cover: FSM completeness, UX state limits, corrupt placeholder invisibility, caching revocability, structured restore rejection, cross-PD RedactedMetadata, latency bound, 5-op API, DeletedFinal gating, Mesh display limits.

### Customization Exceeded Hypothesis (condensed)

| Rival Advantage | SexOS-Native Fix | Invariant/Proof Gate |
|----------------|------------------|---------------------|
| Rich card layouts/grids | View prefs UI-only: list/grid/cards/compact. Theme tokens compiled. No plugins. | §15.45-46 | F9+F3 |
| Powerful sorting/grouping | Sort/group dimensions metadata-redacted. E8 protects privacy. | §15.37-38 | F7+E8 |
| Rich templates with permissions | Template = metadata-only. No grants. Each doc needs individual OpenIntent. | §15.40, §15.39 | F4+E3 |
| Clean restore timeline | Tombstone visibility configurable. RestoreIntent returns structured rejection. | §15.41-42 | F6 |
| App open-with persistence | OpenPreference is hint. Capability check mandatory. Session caching. | §15.39, §15.29 | F8+E3 |
| Rich icon sets/themes | No user-created packs. Tokens bounded. Cannot claim authority. Accessibility required. | §15.46, §15.48 | F9 |
| Accessibility-aware customization | Visual-only customizations need accessible alternative. `[linen.pref.accessibility.warn]` if missing. Reset restores. | §15.48, §15.47 | F9+F7 |

---

## 23. Customization Policy

**Core principle:** Linen must be deeply customizable as a user-facing workspace, but customization must be capability-scoped, validated, reversible, accessible, and unable to customize away storage, privacy, lifecycle, or authority invariants.

### Customizable (14 domains) + Constraint

View mode: list/grid/cards/compact (UI-only, no lifecycle effect). Sorting: name/type/modified/project/custom (metadata-redacted when revealing private fields). Grouping: project/type/tag/status (display-only). Color/icon/accent: bounded compiled theme tokens (no identity/authority claim). Project templates: metadata presets only (no grants). Metadata display: hidden/redacted/summary/full (gated by E8). OpenIntent: ask/last app/preferred hint (always capability-checked). App association: per ObjectKind (hint only, not permission). Tombstone visibility: hidden/archive/restore view (cannot make live). Restore confirmation: level configurable (cannot bypass validation). Draft retention: duration policy (blocked from durable until E gates). Proof verbosity: detail level (cannot suppress required safety markers). Project sidebar: order/pinning/visibility (UI-only). Import/export: preference only (STOP FIRST until trust/storage policy).

### NOT Customizable (21 hard boundaries, condensed)

Capability/OpenIntent/RestoreIntent validation → never bypassable. Delete→state transition, not POSIX unlink. DeletedFinal terminal. Project membership → metadata-only, no grant. DocumentRef/ProjectRef → not path/directory authority. E-owned: key namespace/range, sequence/schema/version, corruption detection, privacy redaction minimums, E9 durable gate. Collar grants → Linen does not mediate. Mesh/Bell/Silk/Quil/sexdisplay → PD boundaries hard (Mesh visualizes, Bell notifies, Quil checks, Silk renders, sexdisplay pixels-only). Required proof markers → verbosity cannot suppress. No raw cross-PD pointers/shared buffers.

### Preference Lifecycle (6 steps)

1. **Load** → `[linen.pref.load]`. 2. **Validate** → `[linen.pref.validate.ok]` or `.reject`. 3. **Apply** → `[linen.pref.apply]` (UI prefs immediate; policy prefs may need capability re-validation). 4. **Persist** → `[linen.pref.persist.reject]` if before E gates. 5. **Redact** → `[linen.pref.redact]` per E8. 6. **Reset** → `[linen.pref.reset]` restores compiled defaults.

### Ownership

Preferences are user/shell/Linen-policy owned — not app-owned. Apps cannot define or override. Shell provides UI but validates through Linen API. Cross-PD sync uses RedactedMetadata only.
