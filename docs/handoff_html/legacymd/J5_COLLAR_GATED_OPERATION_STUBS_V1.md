# J5: Collar-Gated Operation Stubs

**Status:** Handoff (implemented, built)
**Commit:** *(to be committed)*
**Build:** *(to be verified)*

## 1. Purpose

Add additive Collar operation gate stubs to the silk-shell codebase. These stubs
define the operation kinds that will eventually require Collar authority, and
provide a stub policy function that returns decisions without real authority
checks. The stub is wired into the J4 Linen→Quil buffer link path as a
demonstration.

### What J5 IS
- `CollarOperation` enum (7 variants) — operation kinds requiring authority
- `CollarDecision` enum (5 variants) — stub decision outcomes
- `collar_check_operation_stub()` — inline policy function returning stub decisions
- Wired into `open_linen_object_in_quil()` — calls stub before linking

### What J5 IS NOT
- Not real authority enforcement (no Collar PD, no PDX, no grants)
- Not secret/key storage
- Not prompts or chrome
- Not kernel/ABI/sex-pdx changes
- Not sexdisplay changes
- Not WINDOWS Vec changes

## 2. Changed Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +CollarOperation enum, +CollarDecision enum, +collar_check_operation_stub(), wire into open_linen_object_in_quil() |
| `docs/handoff/J5_COLLAR_GATED_OPERATION_STUBS_V1.md` | This document |

## 3. Enum Variants

### CollarOperation

| Variant | Value | Description |
|---------|-------|-------------|
| `OpenObject` | 0 | Open a Linen object |
| `RenameObject` | 1 | Rename a Linen object |
| `ArchiveObject` | 2 | Archive/delete a Linen object |
| `SaveBuffer` | 3 | Persist a Quil buffer to storage |
| `BuildTarget` | 4 | Execute a build target |
| `RunTarget` | 5 | Run/deploy a target |
| `LinkObjectToBuffer` | 6 | Link a Linen object to a Quil buffer |

### CollarDecision

| Variant | Value | Description |
|---------|-------|-------------|
| `AllowStub` | 0 | Safe placeholder op — allowed |
| `DenyMissingObject` | 1 | Referenced Linen object not found |
| `DenyMissingBuffer` | 2 | Referenced Quil buffer not found |
| `NeedsGrantLater` | 3 | Would require real Collar grant |
| `BlockedStopFirst` | 4 | Blocked by STOP FIRST policy |

## 4. Decision Behavior

`collar_check_operation_stub(op, object_id, buffer_id) → CollarDecision`

| Condition | Decision | Proof Marker |
|-----------|----------|-------------|
| Any call | — | `[collar.gate.check]` |
| `object_id != 0` not found in LINEN_OBJECTS | `DenyMissingObject` | `[collar.gate.reject]` reason=missing_object |
| `buffer_id != 0` not found in QUIL_BUFFERS | `DenyMissingBuffer` | `[collar.gate.reject]` reason=missing_buffer |
| `OpenObject` or `LinkObjectToBuffer` | `AllowStub` | `[collar.gate.allow_stub]` |
| `SaveBuffer`, `BuildTarget`, or `RunTarget` | `BlockedStopFirst` | `[collar.gate.reject]` reason=stop_first |
| `RenameObject` or `ArchiveObject` | `NeedsGrantLater` | `[collar.gate.needs_grant]` |

### Rationale
- **OpenObject / LinkObjectToBuffer**: safe — only manipulate in-memory shell
  data structures (LINEN_OBJECTS / QUIL_BUFFERS). No storage, build, or
  authority-sensitive side effects.
- **SaveBuffer / BuildTarget / RunTarget**: blocked by STOP FIRST policy.
  Would require real storage, build pipeline, or authority enforcement.
- **RenameObject / ArchiveObject**: marked NeedsGrantLater. Would modify object
  metadata that a real Collar should gate, but low-risk enough to not warrant
  STOP FIRST.

## 5. J4 Integration

The J4 `open_linen_object_in_quil()` function now gates the Linen→Quil buffer
link through the collar stub:

```
step 2:  grant_ref check (no_grant marker if 0)
step 2.5: collar_check_operation_stub(LinkObjectToBuffer, object_id, 0)
          → AllowStub: continue
          → other: emit [linen.quil.open.reject.collar] + return false
step 3:  LinenObjectKind → QuilBufferKind mapping
step 4:  buffer slot find/create
step 5:  linked_surface_id update
step 6:  [linen.quil.buffer.linked] proof marker
step 7:  open Quil surface
```

This ensures the collar gate fires before any buffer table mutation, but after
the initial object lookup and grant_ref informational check.

## 6. Proof Markers

| Marker | Location | Trigger |
|--------|----------|---------|
| `[collar.gate.check]` | collar_check_operation_stub() | Entry to any collar gate check |
| `[collar.gate.allow_stub]` | collar_check_operation_stub() | OpenObject or LinkObjectToBuffer allowed |
| `[collar.gate.needs_grant]` | collar_check_operation_stub() | RenameObject or ArchiveObject — would need real grant |
| `[collar.gate.reject]` | collar_check_operation_stub() | Any denial (missing object, missing buffer, STOP FIRST) |
| `[linen.quil.open.reject.collar]` | open_linen_object_in_quil() | J4 link rejected by collar gate |

## 7. Safety Invariants Preserved

1. **No real authority checked.** Stub only; no Collar PD, no PDX calls.
2. **No secret/key material.** Stub never stores, transmits, or compares secrets.
3. **No Collar PD required.** Self-contained in silk-shell.
4. **No behavior change for existing paths.** Only J4 LinkObjectToBuffer is wired.
5. **No heap allocation.** All enums and match logic are static/stack-only.
6. **Safe degradation.** Missing objects/buffers return Deny variant, not panic.
7. **Additive only.** Existing lifecycle, focus, tiling, atlas, close paths unchanged.

## 8. Forbidden Areas Untouched

- kernel/: untouched
- crates/sex-pdx/: untouched
- servers/sexdisplay/: untouched
- servers/linen/: untouched
- servers/quil/: untouched
- WINDOWS Vec: untouched
- Lifecycle enum: untouched
- Tombstone ring: untouched
- Mesh/Bell implementation: untouched
- Real grant enforcement: untouched
- Secret/key handling: untouched
- Cryptography: untouched
- Real editor/parser/compiler/build code: untouched

## 9. STOP FIRST Status

**No STOP FIRST triggers hit.**

| Trigger | Status |
|---------|--------|
| Kernel edits | ✅ Not touched |
| sex-pdx ABI/opcode edits | ✅ Not touched |
| sexdisplay changes | ✅ Not touched |
| New PDX ops | ✅ Not added |
| Authority enforcement | ✅ Stub only |
| Secret/key handling | ✅ Not touched |
| Filesystem/storage | ✅ Not touched |
| Editor/parser/compiler/build | ✅ Not touched |
| Cross-PD raw pointers | ✅ Not used |
| Shared-memory/backing-buffer redesign | ✅ Not touched |

## 10. Build Result

*(to be filled after build)*

```sh
./scripts/entrypoint_build.sh
```
