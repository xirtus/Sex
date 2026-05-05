# BELL_SERVER_STUB_PLAN_V1

**Status:** Docs-only implementation plan. No code changed.
**Build:** N/A (no code changes).
**Date:** 2026-05-05
**Depends on:** `BELL_EVENT_MODEL_DESIGN_GATE_V1.md`, `BELL_CAPABILITY_POLICY_V1.md`, `BELL_PDX_PROTOCOL_SPEC_V1.md`

---

## 1. Purpose

Plan the smallest safe future Bell server stub implementation. The first Bell implementation should only prove that Bell PD boots, listens on its PDX slot, and rejects unknown messages safely. No real notify protocol, no rendering, no storage, no callbacks, no sound, no app integration.

### Why a plan before implementation

The Bell server stub crosses several STOP FIRST boundaries simultaneously: naming, slot allocation, opcode assignment, kernel spawn path, and PDX cap grants. This plan documents those gates so the future implementation phase can proceed without repeated design review.

---

## 2. Proposed Implementation Scope (Future Phase)

When `BELL_SERVER_STUB_V1` is greenlit, the following files are candidates for change. **No files are created or edited in this plan phase.**

### Candidate files

| File | Change | Gate Required |
|------|--------|---------------|
| `servers/sexbell/Cargo.toml` | New crate | Naming decision |
| `servers/sexbell/src/main.rs` | New server binary | All gates |
| `workspace/Cargo.toml` | Add `sexbell` workspace member | Naming decision |
| `kernel/src/init.rs` | Add sexbell spawn + cap grant | Slot + cap gate |
| `crates/sex-pdx/src/lib.rs` | Add `SLOT_BELL`, `OP_BELL_*` constants | Protocol/opcode gate |
| `sexos_build_spec.toml` or equivalent | Add sexbell build target | Build gate |

### Files explicitly NOT in scope

- `servers/sexdisplay/` — no renderer changes
- `servers/sexstore/` — no storage changes
- `servers/silk-shell/` — no shell changes (except potential spawn dep)
- `crates/` other than sex-pdx — no new crates beyond sex-pdx constant additions
- `kernel/` other than init.rs — no kernel redesign

---

## 3. Naming Decision Gate

### Candidates

| Name | Convention | Pros | Cons |
|------|-----------|------|------|
| `sexbell` | System-service prefix (`sexstore`, `sexdisplay`, `sexgemini`) | Consistent with existing server naming convention; `sex-` prefix identifies system servers | Slightly longer crate name |
| `bell` | Plain name | Shorter, cleaner, matches product name | Inconsistent with existing convention; may conflict with third-party crates |

### Existing conventions

| Server | Crate Name | Directory |
|--------|-----------|-----------|
| sexstore | `sexstore` | `servers/sexstore/` |
| sexdisplay | `sexdisplay` | `servers/sexdisplay/` |
| sexgemini | `sexgemini` | `servers/sexgemini/` |
| sexusb | `sexusb` | `servers/sexusb/` |
| sexshop | `sexshop` | `servers/sexshop/` |
| silk-shell | `silk-shell` | `servers/silk-shell/` |

### Recommendation

**Server crate:** `sexbell` (following `sexstore`/`sexdisplay`/`sexgemini` convention).
**Product/UI name:** Bell (user-facing name, appears in SilkBar, docs, and UI strings).
**Directory:** `servers/sexbell/`.

This matches the pattern where `sex-` prefix = system server binary, while the product name is plain English.

**STOP FIRST** before creating the crate if naming is not explicitly approved.

---

## 4. Spawn and Capability Gate

The following decisions must be made before the server stub is implemented. They are documented here so the future implementation phase can proceed without redesign.

### PD identity

| Property | Proposed Value | Rationale |
|----------|---------------|-----------|
| PD ID / Domain | `9` (next after sexstore's domain 8) | Sequential domain IDs in init.rs |
| PKEY | `9` (matching domain) | 1:1 domain-to-PKEY mapping |
| Spawn order | After sexstore, before silk-shell | Bell may provide event context to shell on boot |

### Slot

| Property | Proposed Value | Rationale |
|----------|---------------|-----------|
| `SLOT_BELL` | `11` (next after `SLOT_SEXSTORE=10`) | Sequential slot IDs in sex-pdx |
| Slot name | `SLOT_BELL` | Consistent with `SLOT_DISPLAY`, `SLOT_SEXSTORE` |

**STOP FIRST** before assigning `SLOT_BELL` — must verify no slot collision and update kernel cap table.

### Opcodes

No opcodes are assigned in the server stub phase. The stub only needs to:
- Open its PDX listen slot
- Reject all incoming messages with `[bell.unknown.reject]`
- Not parse any payload

### Cap grants

| Grant | Source | Target | Gate |
|-------|--------|--------|------|
| `SLOT_BELL` cap | Kernel | sexbell | Slot assignment gate |
| `SLOT_SEXSTORE` read cap | Kernel | sexbell (if needed for policy storage) | Persistence gate (future) |
| `SLOT_DISPLAY` cap | Kernel | sexbell | Rendering gate (future) |

The server stub needs **only** `SLOT_BELL` cap. No other caps are granted at boot.

### Which PDs may notify Bell

In the stub phase: **none**. The stub rejects all messages. When `OP_BELL_NOTIFY` is implemented later, the set of allowed sender PDs is determined by the cap table (same mechanism as sexstore's caller_pd-based cap check).

---

## 5. Minimal Stub Behavior

The future `BELL_SERVER_STUB_V1` should implement the following behavior and **nothing more**:

### On boot

```
1. Emit [bell.boot] marker
2. Open PDX listen on SLOT_BELL
3. Enter main loop: listen → dispatch → reply → yield
```

### Main loop

```
loop {
    match pdx_listen() {
        Ok(msg) => {
            [bell.unknown.reject] opcode={} sender_pd={}
            pdx_reply(DENIED)
        }
        Err(_) => continue,
    }
    pdx_yield();
}
```

### Explicitly NOT implemented in stub

| Feature | Reason |
|---------|--------|
| OP_BELL_NOTIFY parsing | Protocol gate not approved |
| Lane derivation | Cap policy gate not crossed |
| Ring buffer allocation | Event model not implemented |
| sender_pd cap check | Cap table not populated |
| Any private content parsing | No title/body fields in protocol |
| sexdisplay calls | Renderer gate not crossed |
| sexstore calls | Storage gate not crossed |
| SilkBar integration | SilkBar phase not started |
| Action callbacks | Action cap gate not crossed |
| Sound | Audio gate not crossed |
| Heap allocation | No_std invariant |

### Safety requirements

- No panic on unknown message
- No fault on malformed payload (don't parse — just reject)
- No private content in any proof marker
- No heap allocation
- No sexdisplay/sexstore/silk-shell calls
- Bounded listen loop (pdx_yield after each iteration)

---

## 6. Proof Gates

When the stub is implemented, the following proof must be demonstrated:

| Gate | Evidence | Method |
|------|----------|--------|
| Build | ISO produced, no errors | `./scripts/entrypoint_build.sh` |
| Boot | sexbell PD spawns | QEMU boot log: `[bell.boot]` marker |
| Listen | sexbell accepts PDX slot | `[bell.slot.open]` marker (or similar) |
| Reject | Unknown message returns DENIED | `[bell.unknown.reject]` marker |
| No fault | No panic/spin in sexbell | Boot log clean |
| No private content | No title/body in markers | `rg "title\|body\|sender_name"` on sexbell source |
| No renderer calls | No sexdisplay PDX calls | `rg "SLOT_DISPLAY\|0xEC\|0xEF"` on sexbell source |
| No storage calls | No sexstore PDX calls | `rg "SLOT_SEXSTORE\|0xB0\|0xB1\|0xB2"` on sexbell source |

---

## 7. STOP FIRST Gates

**STOP FIRST** before any of the following in the server stub implementation phase:

1. Deciding `bell` vs `sexbell` naming without explicit approval.
2. Assigning a numeric value to `SLOT_BELL`.
3. Assigning numeric values to `OP_BELL_*`.
4. Editing `sex-pdx` for any Bell constant.
5. Editing kernel `init.rs` spawn path.
6. Granting caps to/from sexbell beyond `SLOT_BELL`.
7. Accepting app notifications (implementing `OP_BELL_NOTIFY`).
8. Adding RAM event queue / ring buffer.
9. Adding persistence (sexstore/sexshop calls).
10. Adding display/SilkBar integration.
11. Adding action callbacks.
12. Adding sound.
13. Adding private title/body transport (protocol V1 explicitly forbids this).

---

## 8. Future Phase Split

When implementation begins, the work should be split into these phases (each with its own STOP FIRST review):

| Phase | Scope | Type | Depends On |
|-------|-------|------|------------|
| **BELL_SLOT_OPCODE_ASSIGNMENT_V1** | Assign SLOT_BELL=11, OP_BELL_NOTIFY=0xC0 (etc.) in sex-pdx | Code (sex-pdx only) | This plan + naming approval |
| **BELL_BOOT_SPAWN_V1** | Add sexbell to init.rs spawn sequence, grant SLOT_BELL cap | Code (kernel/init.rs) | Slot assignment |
| **BELL_SERVER_STUB_V1** | Minimal sexbell binary: boot, listen, reject unknown | Code (servers/sexbell/) | Boot spawn |
| **BELL_UNKNOWN_REJECT_PROOF_V1** | Verify stub behavior via QEMU boot + proof markers | Test | Server stub |
| **BELL_NOTIFY_RAM_QUEUE_V1** | Implement OP_BELL_NOTIFY + bounded ring buffer | Code | Server stub + cap policy |
| **BELL_SILKBAR_PRESENCE_V1** | Compact lane-summary indicator in global bar | Code | Notify queue |
| **BELL_INBOX_ROWS_V1** | Full inbox surface adopting SILK_LIST_ROW_VISUAL_CANON | Code | Server stub + canon |

---

## 9. Recommended Next Action

### Before any code

1. **Approve naming**: `sexbell` for server crate, `Bell` for product/UI name.
2. **Approve slot/opcode plan**: `SLOT_BELL=11`, `OP_BELL_NOTIFY=0xC0`, etc. (verify no conflicts).
3. **Approve domain/PKEY/spawn order**: domain 9, PKEY 9, spawn after sexstore.

### Then implement in order

```
BELL_SLOT_OPCODE_ASSIGNMENT_V1   → sex-pdx constants only
BELL_BOOT_SPAWN_V1               → init.rs spawn + cap grant
BELL_SERVER_STUB_V1              → sexbell binary (boot + listen + reject)
BELL_UNKNOWN_REJECT_PROOF_V1     → verify via QEMU boot
```

Each phase has its own STOP FIRST gate. Do not skip phases.

---

## References

- `BELL_EVENT_MODEL_DESIGN_GATE_V1.md` — event model, lanes, privacy
- `BELL_CAPABILITY_POLICY_V1.md` — default-deny capability policy
- `BELL_PDX_PROTOCOL_SPEC_V1.md` — protocol opcodes, message shapes, validation flow
- `SILK_LIST_ROW_VISUAL_CANON_V1.md` — canon for future inbox
- `E15_STORAGE_DOCS_CLEANUP_V1.md` — storage canon
- `kernel/src/init.rs` — reference for existing spawn order (sexstore domain 8)
- `crates/sex-pdx/src/lib.rs` — reference for SLOT_* constants (SLOT_SEXSTORE=10)
- `servers/sexstore/src/main.rs` — reference for PDX listen/dispatch pattern

---

*End of BELL_SERVER_STUB_PLAN_V1.md*
