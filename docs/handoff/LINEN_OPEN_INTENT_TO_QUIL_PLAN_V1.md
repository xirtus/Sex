# LINEN_OPEN_INTENT_TO_QUIL_PLAN_V1

## Status: PLAN (no implementation)

## Current Reality (Audited 2026-05-07)

### What Exists
| Component | Status | Location |
|-----------|--------|----------|
| Quil PD | Boots, surface 201, text buffer, sexfiles probing | `servers/quil/src/main.rs` |
| Collar policy gate | In-process inside silk-shell. No separate PD. | `servers/silk-shell/src/main.rs:~1464` |
| `open_linen_object_in_quil()` | Full implementation: Collar check → buffer create/link → Quil surface open | `servers/silk-shell/src/main.rs:1279` |
| OpenIntent stub (0x46) | Shell sends → Linen validates → Linen replies accepted/error | `servers/linen/src/main.rs:454` |
| PrintScreen→Quil trigger | Works via scancode 0x59 → `OpenObjectInQuil` action → `open_linen_object_in_quil()` | `servers/silk-shell/src/main.rs:11531` |

### The Gap
Enter/Space on selected Linen object → `OP_LINEN_OPEN_INTENT` → Linen replies 0 → **nothing happens next**.
Shell discards the reply. The existing `open_linen_object_in_quil()` path is never invoked.

### Collar Gate Reality
Collar is NOT a separate PD. It's an in-process policy table inside silk-shell:
- `COLLAR_GRANTS: [Option<CollarGrant>; 16]` — grant table
- `collar_check_operation(op, object_id, buffer_id) -> CollarDecision`
- Decisions: AllowStub(1), Allow(2), Deny(3), NeedsGrantLater(4), BlockedStopFirst(5)
- `LinkObjectToBuffer` currently returns `AllowStub` (no real grants exist yet)
- `AccessSexFiles` capability check also returns `AllowStub`

## Safest Architecture

### Option A: Direct Shell Bridge (RECOMMENDED for V1)
```
Enter/Space (Linen focused)
  → OP_LINEN_OPEN_INTENT (0x46) to Linen
  → Linen validates, replies 0
  → Shell checks reply, calls open_linen_object_in_quil(object_id)
  → Collar gate (in-process) checks LinkObjectToBuffer
  → Collar gate checks AccessSexFiles
  → QuilBuffer created/reused
  → Quil surface opened
```

**Pros:**
- Minimal new code (connect existing paths)
- Collar gate already in place for authorization
- Quil PD already boots and handles buffer display
- Enter/Space is the natural activation key (matches user expectation)

**Cons:**
- Shell is the integration point (not ideal long-term)
- No real Collar PD mediates the authority transfer (AllowStub only)
- No file content grant yet (buffer is name/kind only, no data)

### Option B: Linen-Mediated Route (deferred to V2+)
```
Enter/Space
  → OP_LINEN_OPEN_INTENT to Linen
  → Linen validates via SESSION
  → Linen calls Collar PD (doesn't exist yet)
  → Linen calls Quil PD with bounded object metadata
  → Quil opens buffer
```

**Pros:** Cleaner authority model. Linen owns the object → Collar mediates → Quil receives.

**Cons:** Requires Collar as separate PD (doesn't exist). More PDX round-trips. Higher latency.

### Option C: OpenIntent Only, No Quil Connection (deferred)
Keep 0x46 as an intent marker only. Connect to Quil when Collar PD exists.
This preserves the current state and avoids building on AllowStub.

## Recommendation: Option A with STOP FIRST Guard

### Phase A: LINEN_OPEN_INTENT_TO_QUIL_STUB_V1
Connect the existing OpenIntent reply path to `open_linen_object_in_quil()`.

**Changes (approximate):**
```
servers/silk-shell/src/main.rs:
  - In Linen keyboard intercept (Enter/Space block):
    - After pdx_call(SLOT_LINEN, OP_LINEN_OPEN_INTENT, ...), sync-reply and check status
    - If status == 0 (accepted), call open_linen_object_in_quil(obj_id)
    - Log [linen.open_intent.to_quil] id=N status=S
  - No new PDX routes
  - No new opcodes

servers/linen/src/main.rs:
  - No changes needed (0x46 handler is already correct)
```

**Proof markers added:**
```
[linen.open_intent.to_quil] id=N status=0       ← Shell routed intent to Quil
[linen.open_intent.to_quil.skip] id=N status=S  ← Shell received non-zero status
[linen.quil.open.request] id=N                  ← existing marker
[linen.quil.buffer.linked] object_id=N buffer_id=B kind=K  ← existing marker
```

**What this does NOT do:**
- No file content transfer (object metadata only: id, kind, name)
- No real Collar authority grant (AllowStub continues)
- No sexfiles file handle handoff to Quil
- No persistence of the link

**STOP FIRST boundaries:**
1. `collar_check_operation` already gates `LinkObjectToBuffer` → if it returns Deny, the Quil open is blocked. No new bypass.
2. `open_linen_object_in_quil` already handles buffer collision, full table, missing object. No new failure modes.
3. Quil PD already handles buffer display. No Quil changes needed.
4. No kernel/ABI/sex-pdx changes.

### Phase B: COLLAR_OPEN_GRANT_PLAN_V1 (separate phase)
Replace `AllowStub` with real grant table entries for Linen objects → Quil buffers.
This is an authority model upgrade, not a path change. Can be done after Phase A.

### Phase C: Real File Content Grant (deferred further)
Transfer actual file content from SexFiles → Quil buffer via bounded PDX reads.
Requires: Collar mediates the cap transfer, Quil receives a read cap, Linen provides the sexfiles_object_id.

## Implementation Sequence

```
1. LINEN_OPEN_INTENT_TO_QUIL_STUB_V1   ← THIS PLAN
   Connect 0x46 reply to open_linen_object_in_quil()
   Enter/Space opens selected object in Quil
   Proof: [linen.open_intent.to_quil] fires

2. COLLAR_OPEN_GRANT_PLAN_V1
   Replace AllowStub with real grant table entries
   Linen objects get explicit Collar grants for Quil buffers
   Proof: [collar.grant.create] fires on first open

3. LINEN_TO_QUIL_CONTENT_GRANT_V1
   Transfer file content from SexFiles to Quil
   Linen provides sexfiles_object_id → Quil reads via sexfiles PDX
   Proof: file bytes appear in Quil buffer

4. LINEN_PUSH_INVALIDATE_PLAN_V1
   Push-based invalidation when objects change
   Linen notifies Quil of stale buffers
   Proof: [linen.invalidate.push] fires
```

## Exact Next Prompt

```bash
cat > /tmp/linen_open_intent_to_quil_stub_v1.prompt <<'EOF'
MISSION: LINEN_OPEN_INTENT_TO_QUIL_STUB_V1

Connect the OpenIntent reply path to the existing open_linen_object_in_quil().
Enter/Space on selected Linen object opens it in Quil (no file content yet).

BACKUP BEFORE CHANGES.
If something goes wrong: READ HANDOUTS and .mds first.
Reduce token waste: rg first, inspect small snippets only, no broad dumps.
Save recurring fixes/issues in docs/handoff.

NO Linux assumptions. NO POSIX.
Strict no_std Rust Sex Microkernel:
- no std/libc/threads
- PDX only
- MPK/PKU/PKEY isolation  
- no kernel/ABI/sex-pdx edits
- no sexdisplay edits
- sexdisplay sole framebuffer writer
- preserve framebuffer bounds checks
- no broad refactor
- no cross-PD raw pointers

CURRENT:
- OP_LINEN_OPEN_INTENT = 0x46 fires on Enter/Space when Linen focused.
- Linen validates object_id via SESSION.get(object_id, 0) and replies 0 or error.
- Shell currently discards the reply.
- open_linen_object_in_quil() exists and works via PrintScreen (0x59).
- Collar gate (in-process) mediates LinkObjectToBuffer and AccessSexFiles.

PATCH SCOPE:
Prefer only:
- servers/silk-shell/src/main.rs
- docs/handoff/LINEN_OPEN_INTENT_TO_QUIL_STUB_V1.md

NO:
- kernel edits
- sex-pdx edits
- sexdisplay edits
- sexfiles edits
- real file content transfer
- Collar PD (doesn't exist)
- new opcodes
- new PDX routes

IMPLEMENTATION:
In the Linen keyboard intercept (Enter/Space block in silk-shell):
1. After pdx_call(SLOT_LINEN, OP_LINEN_OPEN_INTENT, ...), capture the reply via linen_sync_reply()
2. If reply == 0 (Linen accepted the intent):
   - Call open_linen_object_in_quil(obj_id)
   - Log [linen.open_intent.to_quil] id=N
3. If reply != 0 or error:
   - Log [linen.open_intent.to_quil.skip] id=N status=S
   - Do NOT call open_linen_object_in_quil
4. If obj_id == 0 (no selection):
   - Keep existing [linen.open_intent.skip] logic
   - Do NOT attempt Quil open

The existing Collar gate inside open_linen_object_in_quil() handles:
- AccessSexFiles capability check
- LinkObjectToBuffer authorization
- Buffer collision safety
- Buffer full safety
No new safety checks needed.

PROOF:
- ./scripts/entrypoint_build.sh
- Boot all 6 PDs
- Focus Linen surface 200
- J/K selects an object
- Enter or Space sends OpenIntent → Linen accepts →
  [linen.open_intent.to_quil] id=N → Quil surface opens with object linked
- Verify [linen.quil.buffer.linked] and [collar.policy.check] markers
- Verify no crash, no fault, no panic
- Verify Quil surface 201 is visible and shows the linked buffer

HANDOFF:
Write docs/handoff/LINEN_OPEN_INTENT_TO_QUIL_STUB_V1.md

RETURN:
1. diff summary
2. proof markers
3. next recommended phase
EOF
```
