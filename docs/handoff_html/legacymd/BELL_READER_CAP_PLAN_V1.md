# BELL_READER_CAP_PLAN_V1

**Status:** Docs-only plan. No code changed.
**Build:** N/A (no code changes).
**Date:** 2026-05-05
**Depends on:** `BELL_LIST_SUMMARY_FREEZE_V1.md`, `BELL_CAPABILITY_POLICY_V1.md`, `BELL_PDX_PROTOCOL_SPEC_V1.md`

---

## 1. Purpose

Design the smallest safe authority path for a real Bell reader to call `OP_BELL_LIST`. Currently, only the kernel (via direct message enqueue) can send `OP_BELL_LIST` to sexbell — the read-cap allowlist is empty, meaning every real sender is rejected at the sexbell level within the existing handler.

This plan defines **one controlled reader grant**: silk-shell receives `SLOT_BELL` routing cap, sexbell enforces a static allowlist for `OP_BELL_LIST`, and proof verifies both acceptance and rejection.

**No implementation.** This plan only. STOP FIRST gates apply before any code change.

---

## 2. First Reader Decision: silk-shell

### Options considered

| Option | Description | Verdict |
|--------|-------------|---------|
| **A: silk-shell (domain 3)** | Grant SLOT_BELL routing cap to silk-shell, add sexbell-side allowlist for OP_BELL_LIST | **PREFERRED** — policy owner, already has 5 caps, can push summaries to SilkBar later |
| **B: SilkBar (domain 6)** | Grant SLOT_BELL to SilkBar for direct read | Rejected — SilkBar v7 is producer stub, no PDX transport yet, no policy authority |
| **C: Both** | Grant to both silk-shell and SilkBar | Rejected for V1 — scope creep, two-phase authority model needed |
| **D: No grant (kernel-only)** | Keep kernel as sole OP_BELL_LIST sender | Rejected — defeats purpose of real reader path |

### Decision: silk-shell (domain 3)

**Rationale:**
- Already the policy owner for scenes, accessibility, command palette, Quil routing, storage
- Already has 5 cap grants (SLOT_DISPLAY, SLOT_SHELL, SLOT_SILKBAR, SLOT_SEXSTORE, SLOT_QUIL)
- Can call `OP_BELL_LIST` at any time and push event summaries to SilkBar via existing SLOT_SILKBAR cap
- SilkBar does NOT have PDX transport yet (local producer stub, v7) — granting reader caps to SilkBar now would force premature PDX design
- One reader is the minimal safe surface: default-deny for all other PDs

### What silk-shell can do with read authority

| Action | Allowed? | Notes |
|--------|----------|-------|
| Call `OP_BELL_LIST` with lane_filter | ✅ Yes | Summary-only, no private content |
| Call `OP_BELL_NOTIFY` | ❌ No (no caps granted yet) | Separate notify cap phase |
| Call `OP_BELL_CLEAR` | ❌ No | Not implemented |
| Call `OP_BELL_ACTION` | ❌ No | Not implemented |
| Call `OP_BELL_CLOSE` | ❌ No | Not implemented |
| Call `OP_BELL_SUBSCRIBE` | ❌ No | Not implemented |
| Call `OP_BELL_SET_POLICY` | ❌ No | Not implemented |
| Call `OP_BELL_MUTE_SENDER` | ❌ No | Not implemented |
| Mutate queue | ❌ No | Read-only |
| Access private content | ❌ No | Summary fields only |

### What this enables

After this phase, silk-shell can call `OP_BELL_LIST` and receive event count + summaries. This is the prerequisite for:
- SilkBar presence (lane-count indicator)
- Inbox row rendering (adopting SILK_LIST_ROW_VISUAL_CANON)
- Event-driven scene/accessibility responses

---

## 3. Authority Model

### Two-layer enforcement

```
Layer 1: Kernel cap grant (init.rs)
  └── SLOT_BELL → silk-shell
      Allows silk-shell to route messages to sexbell's message ring.
      This is a routing cap only — does not authorize specific opcodes.

Layer 2: sexbell-side allowlist (sexbell/main.rs)
  └── Static allowlist of domain IDs permitted to call OP_BELL_LIST.
      Default-deny: all unlisted PDs (including kernel with caller_pd=0).
      Checked before queue read.
```

### Why two layers?

| Layer | What it prevents | What it does NOT prevent |
|-------|-----------------|-------------------------|
| Kernel cap (SLOT_BELL) | Unauthorized PDs cannot send any message to sexbell | Does not distinguish between opcodes |
| sexbell allowlist | Authorized PDs can only call allowed opcodes | Must be maintained as ops grow |

Without the sexbell allowlist, any PD with `SLOT_BELL` (currently only sexbell itself) could call any opcode. The sexbell allowlist ensures that even a PD with the routing cap cannot call `OP_BELL_NOTIFY`, `OP_BELL_CLEAR`, etc. without explicit per-opcode authorization.

### Static allowlist (V1)

```rust
/// Static allowlist of PDs permitted to call OP_BELL_LIST.
/// Default-deny: any PD not in this list is rejected.
/// Extended in future phases as new readers are approved.
const BELL_LIST_ALLOWLIST: &[u32] = &[
    3,  // silk-shell (domain 3, policy owner)
];
```

### Enforcement point

Checked AFTER argument validation but BEFORE queue iteration:

```
OP_BELL_LIST
  ├── Parse + validate lane_filter, max_results
  │     └── Invalid → [bell.list.reject] → continue
  ├── Check caller_pd against BELL_LIST_ALLOWLIST
  │     ├── Not found → [bell.readcap.deny] reason=no_read_cap → continue
  │     └── Found    → [bell.readcap.allow] (budget-limited)
  ├── [bell.list.recv]
  ├── Read queue, emit items
  └── [bell.list.done] or [bell.list.empty]
```

Rationale for checking AFTER validation:
- Validation errors are protocol errors, not authorization errors — they should return `[bell.list.reject]` regardless of caller
- `[bell.readcap.deny]` is specifically for authorization failures
- Unapproved callers with invalid args get `[bell.list.reject]` (they never reach the allowlist check)
- This matches the principle that protocol errors are distinct from authorization errors

---

## 4. Grant Path (kernel init.rs)

### Current state

```rust
// Line 169-177: sexbell self-cap only
pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
```

### After grant

```rust
// sexbell self-cap (existing)
pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));

// silk-shell read-cap grant
pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(silkshell_id));
```

Wait — this is wrong. `pd` in the sexbell block is `DOMAIN_REGISTRY.get(sexbell_id)`. The silk-shell grant needs to be in the silk-shell cap block.

The grant should be added to the existing silk-shell cap block (lines 95-108):

```rust
// Existing
pd.grant_capability(sex_pdx::SLOT_DISPLAY, CapabilityData::Domain(sexdisp_id));
pd.grant_capability(sex_pdx::SLOT_SHELL,   CapabilityData::Domain(silkshell_id));
pd.grant_capability(sex_pdx::SLOT_SILKBAR, CapabilityData::Domain(silkbar_id));
pd.grant_capability(sex_pdx::SLOT_SEXSTORE, CapabilityData::Domain(sexstore_id));

// New addition
pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
```

This grants silk-shell the SLOT_BELL routing cap. When silk-shell calls `pdx_call` with slot=SLOT_BELL (12), the kernel routes the message to sexbell's message ring.

### Key guarantee

The kernel cap layer ensures that `caller_pd` is always set to the sender's actual domain ID. Silk-shell cannot spoof its identity. sexbell's allowlist check uses the authoritative `msg.caller_pd` which is kernel-set and immutable from userspace.

---

## 5. Proof Plan

### Phase A: Positive proof (silk-shell calls OP_BELL_LIST, accepted)

**Set up:**
1. Kernel scaffold enqueues one `OP_BELL_NOTIFY` to seed the queue (caller_pd=0, valid payload)
2. Kernel scaffold enqueues one `OP_BELL_LIST` with `caller_pd=3` (mimicking silk-shell), `lane_filter=0xFF`, `max_results=4`

**Expected markers (queue has 1 event):**

```
[bell.queue.push] id=1 final_lane=0 count=1
[bell.notify.ok] event_id=1

[bell.readcap.allow] caller_pd=3 op=list
[bell.list.recv] lane_filter=0xff max_results=4 caller_pd=3
[bell.list.item] event_id=1 final_lane=0 category=0 privacy=0 redaction=0
[bell.list.done] count=1
```

### Phase B: Negative proof (unapproved PD calls OP_BELL_LIST, rejected)

**Set up:**
1. Kernel scaffold enqueues one `OP_BELL_LIST` with `caller_pd=2` (mimicking sexdrive — unapproved), `lane_filter=0xFF`, `max_results=4`

**Expected markers:**

```
[bell.readcap.deny] caller_pd=2 op=list reason=no_read_cap
```

**Absent:**

```
[bell.list.recv]         ← absent (cut off before queue read)
[bell.list.item]         ← absent
[bell.list.done]         ← absent
[bell.list.empty]        ← absent
[bell.unknown.reject]    ← absent (OP_BELL_LIST is a known opcode)
```

### Both scaffolds temporary, removed after proof.

---

## 6. Exact Markers (New + Existing)

### New markers for read-cap

| Marker | Budget | Fields | When |
|--------|--------|--------|------|
| `[bell.readcap.allow]` | 8 | `caller_pd`, `op` | Approved caller passes allowlist check |
| `[bell.readcap.deny]` | 8 | `caller_pd`, `op`, `reason` | Unapproved caller fails allowlist check |

### Existing markers preserved (unchanged)

| Marker | Budget | Status |
|--------|--------|--------|
| `[bell.list.recv]` | 8 | Preserved |
| `[bell.list.item]` | 16 | Preserved |
| `[bell.list.empty]` | 4 | Preserved |
| `[bell.list.done]` | 8 | Preserved |
| `[bell.list.reject]` | 4 | Preserved |
| `[bell.notify.*]` | Various | Preserved |
| `[bell.queue.*]` | Various | Preserved |

---

## 7. Implementation Sketch (for BELL_READER_CAP_IMPLEMENT_V1)

### sexbell/main.rs additions

1. **Static allowlist constant:**
   ```rust
   const BELL_LIST_ALLOWLIST: &[u32] = &[3]; // silk-shell (domain 3)
   ```

2. **Allowlist helper:**
   ```rust
   fn is_list_reader_allowed(caller_pd: u32) -> bool {
       BELL_LIST_ALLOWLIST.contains(&caller_pd)
   }
   ```

3. **Check in OP_BELL_LIST handler** (after arg validation, before `[bell.list.recv]`):
   ```rust
   // ── Check read-cap allowlist ──
   if !is_list_reader_allowed(caller_pd) {
       unsafe {
           static mut BELL_READCAP_DENY_BUDGET: u32 = 8;
           let b = &mut BELL_READCAP_DENY_BUDGET;
           if *b > 0 {
               *b -= 1;
               serial_println!("[bell.readcap.deny] caller_pd={} op=list reason=no_read_cap",
                   caller_pd);
           }
       }
       continue;
   }

   unsafe {
       static mut BELL_READCAP_ALLOW_BUDGET: u32 = 8;
       let b = &mut BELL_READCAP_ALLOW_BUDGET;
       if *b > 0 {
           *b -= 1;
           serial_println!("[bell.readcap.allow] caller_pd={} op=list", caller_pd);
       }
   }
   ```

### kernel init.rs addition

In the silk-shell cap grant block (lines 95-108), add:

```rust
// Bell read-cap: silk-shell can call OP_BELL_LIST
pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
serial_println!("[kernel.sexbell.cap.shell] shell→bell slot=12");
```

---

## 8. Accepted vs Rejected Callers (V1)

| Caller | Domain | OP_BELL_LIST allowed? | Reason |
|--------|--------|----------------------|--------|
| silk-shell | 3 | ✅ Yes | Policy owner, first reader |
| kernel (direct message) | 0 | ❌ No (unless allowlisted) | No retainable kernel behavior |
| sexdisplay | 1 | ❌ No | No Bell authority |
| sexdrive | 2 | ❌ No | App, no Bell authority |
| sexinput | 4 | ❌ No | Input server, no Bell authority |
| sexusb | 5 | ❌ No | Driver, no Bell authority |
| SilkBar | 6 | ❌ No (V1) | Future: after PDX transport |
| linen | 7 | ❌ No | Surface server, no Bell authority |
| sexstore | 8 | ❌ No | K/V storage, no Bell authority |
| quil | 9 | ❌ No | App surface server, no Bell authority |
| sexbell | 10 | ❌ N/A (self) | sexbell is the service, not a caller |

---

## 9. Non-Targets (Explicitly Excluded from This Plan)

- No SilkBar rendering/presence
- No inbox UI
- No reply ABI
- No persistence beyond RAM queue
- No private content (title, body, sender name, file paths)
- No app-level notification access
- No queue clear/mutation (`OP_BELL_CLEAR`)
- No notify authority for silk-shell (separate phase)
- No action callbacks
- No sound/audio integration
- No multi-reader allowlist design (two readers deferred)
- No changes to sex-pdx constants
- No kernel ABI changes
- No heap/alloc

---

## 10. STOP FIRST Gates

**STOP FIRST** before any of the following:

1. Choosing SilkBar as the first reader (SilkBar has no PDX transport yet — must not force premature PDX design).
2. Granting `SLOT_BELL` to more than one reader in the implementation phase (multi-reader is a separate phase).
3. Adding a reply ABI (`pdx_reply` with data payload) to return structured summaries.
4. Implementing SilkBar presence or inbox UI in the same patch.
5. Allowing apps (sexdrive, etc.) to call `OP_BELL_LIST` without explicit policy review.
6. Using the sandboxed `caller_pd` incorrectly — must be kernel-authoritative, never user-supplied.
7. Adding private content fields (title, body, sender name) to anywhere in the path.
8. Adding `OP_BELL_CLEAR`, `OP_BELL_ACTION`, or any queue-mutation opcode.
9. Retaining the kernel proof scaffold beyond the cleanup phase.
10. Requiring sex-pdx constant changes or kernel ABI edits.

---

## 11. Next Phases (Recommended Order)

| Phase | Scope | Type |
|-------|-------|------|
| **BELL_READER_CAP_IMPLEMENT_V1** | Add allowlist to sexbell, grant SLOT_BELL to silk-shell, add kernel scaffold | Implementation |
| **BELL_READER_CAP_PROOF_V1** | QEMU proof with positive (caller_pd=3) and negative (caller_pd=2) | Proof |
| **BELL_READER_CAP_CLEANUP_V1** | Remove both scaffolds | Cleanup |
| **BELL_READER_CAP_FREEZE_V1** | Freeze read-cap design | Freeze |
| **BELL_SILKBAR_PRESENCE_PLAN_V1** | Design lane-count summary push to SilkBar | Docs |

---

## 12. Security Properties

| Property | How it's enforced |
|----------|-------------------|
| **Default-deny** | Empty allowlist blocks all callers. Only silk-shell added in V1. |
| **Kernel-authoritative caller ID** | `caller_pd` is kernel-set on syscall path, immutable by userspace |
| **Read-only authority** | OP_BELL_LIST only. No notify/clear/mute/action/set-policy. |
| **No private content** | Summary markers expose only StructuralMeta fields |
| **No retained kernel behavior** | Scaffolds removed after proof |

---

## References

- `BELL_LIST_SUMMARY_FREEZE_V1.md` — current list summary state (marker-only, no caps)
- `BELL_CAPABILITY_POLICY_V1.md` — default-deny, sender classes, lane derivation
- `BELL_PDX_PROTOCOL_SPEC_V1.md` — message shapes, caller_pd model
- `kernel/src/init.rs` — cap grant topology, silk-shell current caps
- `servers/sexbell/src/main.rs` — current handler, allowlist insertion point
- `servers/silk-shell/src/main.rs` — reader implementation target
- `crates/sex-pdx/src/lib.rs` — SLOT_BELL=12, OP_BELL_LIST=0xC3

---

*End of BELL_READER_CAP_PLAN_V1.md*
