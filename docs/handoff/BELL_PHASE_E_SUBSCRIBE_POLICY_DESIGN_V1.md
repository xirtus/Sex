# Bell Phase E: SUBSCRIBE / SET_POLICY Design

**Status:** Design only — no implementation, no code changes.

**Date:** 2026-05-06

---

## Summary

Design options for Bell Phase E: `OP_BELL_SUBSCRIBE` (push notification for lane-summary
changes) and `OP_BELL_SET_POLICY` (per-app user policy overrides). Both are cleanly
blocked behind kernel-ABI and persistence-schema STOP-FIRST boundaries.

**Recommended path:** Phase E-1 (no-kernel-change poll optimization) → Phase E-2 (policy) →
Phase E-3 (push IPC, requires kernel change).

---

## 1. Current State

### 1.1 Bell State (V1 Complete)

| Component | Status |
|-----------|--------|
| Queue | 16-entry ring, single-threaded |
| NOTIFY/CLOSE/ACTION/LIST/CLEAR | Implemented |
| MUTE_SENDER | Implemented |
| Spam budget | 64-tick window, 8 max per PD, LRU eviction |
| Privacy redaction | 4 levels (Public → FullHidden) |
| Allowlist | PD 3 (silk-shell) and PD 6 (silkbar) can LIST |
| Reply format | Packed u64: total_visible[7:0], lane_counts[55:8], redacted[63:56] |

### 1.2 SilkBar Presence (V1 Complete)

- SilkBar polls `OP_BELL_LIST` every 2 seconds (`uptime_seconds % 2 == 0`)
- Reply arrives asynchronously via `pdx_try_listen_raw(0)` with `type_id=1, caller_pd=1`
- Packed u32 forwarded to sexdisplay as `UpdateKind::SetBellPresence`
- Bell dot color: dim (no events/denied), amber (redacted events), gold (active events)
- Count badge rendered in top-right of Bell module

### 1.3 Blocked Phase E Opcodes

| Opcode | Constant | In sex-pdx? | Bell handler? | SilkBar handler? |
|--------|----------|-------------|---------------|-------------------|
| SUBSCRIBE | `0xC5` | ✅ Defined | ❌ Falls to `[bell.unknown.reject]` | ❌ Not implemented |
| SET_POLICY | `0xC6` | ✅ Defined | ❌ Falls to `[bell.unknown.reject]` | ❌ Not implemented |

---

## 2. OP_BELL_SUBSCRIBE Design

### 2.1 Goal

SilkBar (or any allowlisted subscriber) registers for push notification when the lane-summary
aggregate changes (a new event is enqueued, an event is dismissed/cleared, or mute list changes).

### 2.2 Option A: No-Kernel Change — Generation Counter Polling (RECOMMENDED FOR E-1)

**Architecture:**
```
Bell maintains a global generation counter (u64).
Incremented on every queue/mute change.
OP_BELL_LIST reply includes the current generation value.
SilkBar polls less aggressively (e.g., every 5s instead of every 2s).
```

**Payload change to LIST reply (extended packed u64):**
```
Current:  total_visible[7:0] | lane_counts[55:8] | redacted[63:56]
Extended: total_visible[7:0] | lane_counts[39:8] | redacted[47:40] | generation[63:48]
```

Or simpler — new dedicated `OP_BELL_SUBSCRIBE` handler that returns just the generation:
```
arg0 = 0 → returns current generation as u64
Caller polls with OP_BELL_SUBSCRIBE every N seconds.
If generation unchanged, skip OP_BELL_LIST (no queue scan).
```

**Pros:**
- No kernel changes
- No push IPC
- Reduces queue scan overhead (O(16) on every poll → O(1) generation check)
- SilkBar still polls, but can reduce cadence from 2s to 5–10s

**Cons:**
- Still polling, not push
- Generation counter must be atomically consistent (single-threaded Bell is fine)
- No wakeup — SilkBar must still run its own loop

**Wire format (minimal extension):**
```
Request:  OP_BELL_SUBSCRIBE (0xC5)
          arg0 = generation_threshold (0 = always respond)
Reply:    u64 = current_generation
```

**Subscriber tracks:** `if new_gen != cached_gen → call OP_BELL_LIST to get actual counts`

### 2.3 Option B: Kernel Push IPC — STOP FIRST

**Architecture:**
```
Bell calls a hypothetical kernel syscall to enqueue a message directly
into the subscriber's message ring:
  kernel_push_reply(target_pd: u32, slot: u64, value: u64)
```

Or: kernel grants a shared-memory notification word between Bell and subscriber.
Bell writes to the word when state changes; subscriber polls the word (not PDX).

**STOP FIRST because:**
- Requires new kernel syscall or ABI change
- Shared-memory notification word requires kernel cap model extension
- No existing mechanism for inter-domain push in the kernel
- Would set a precedent for all future push notification patterns

**If kernel push existed, SUBSCRIBE flow:**
```
1. SilkBar → Bell: OP_BELL_SUBSCRIBE
2. Bell records subscriber PD + slot in a small subscriber table
3. On queue change, Bell calls kernel_push_reply(subscriber, slot, packed)
4. SilkBar's next pdx_try_listen_raw returns the push message
```

**Pros:**
- True push — no polling
- Sub-second notification latency

**Cons:**
- Kernel ABI change
- Requires subscriber table in Bell (bounded: 2–4 entries)
- Lost-wakeup risk (push arrives between SilkBar's listen and poll)
- Reply storm risk (rapid queue changes → floods subscriber)

### 2.4 Option C: One-Shot Wait/Reply — STOP FIRST

**Architecture:**
```
SilkBar calls OP_BELL_SUBSCRIBE with a one-shot semantic.
Bell blocks the handler thread until a queue change occurs, then replies.
No polling needed.
```

**STOP FIRST because:**
- Bell is single-threaded — blocking would stop all event processing
- Would require Bell to spawn a handler thread or use async
- No kernel support for blocking receive with timeout
- Single-threaded server model is deliberate for simplicity

---

## 3. OP_BELL_SET_POLICY Design

### 3.1 Goal

Allow silk-shell (PD 3) to set per-app (per caller_pd) policy overrides:
- Privacy level override (e.g., force an app's events to FullHidden)
- Default lane override
- Mute (replaces OP_BELL_MUTE_SENDER or adds granular mute reasons)

### 3.2 Option A: Volatile Policy Only (RECOMMENDED FOR E-2a)

**Architecture:**
```
Bell maintains a small static policy table in RAM.
Entries: [caller_pd: u32, policy_flags: u8, privacy_override: u8, lane_override: u8]
Table capacity: 8 entries (LRU eviction on overflow).
Policies are VOLATILE — lost on Bell restart.
```

**Wire format:**
```
Request:  OP_BELL_SET_POLICY (0xC6)
          arg0 = caller_pd (target app)
          arg1 = policy_mask | (privacy_override << 8) | (lane_override << 16)
Reply:    u64 = 0 (OK), u64::MAX (error)

Policy mask bits:
  bit 0 = override privacy level
  bit 1 = override lane
  bit 2 = force mute
  bits 8–9 = privacy override value (0–3)
  bits 16–18 = lane override value (0–5)
```

**Pros:**
- No storage dependency
- Simple, bounded, no persistence
- Bell can apply policy at notify time (policy check after mute, before enqueue)

**Cons:**
- Policies lost on Bell restart
- No history, no UI persistence across reboot
- Shell must re-apply policies on each boot

### 3.3 Option B: Sexstore-Backed Policy Schema — STOP FIRST

**Architecture:**
```
Bell reads/writes policy entries to sexstore on SET_POLICY and on boot.
Policy schema in sexstore:
  Key namespace: 0xBELL_POLICY_BASE | caller_pd
  Value: packed policy bytes
Bell loads policies on boot, flushes on SET_POLICY.
```

**STOP FIRST because:**
- Requires sexstore schema design audit
- Bell is currently dependency-free (no sexstore calls)
- Adds sexstore dependency to Bell's critical path (boot block if sexstore is down)
- Sexstore's "bounded system-settings K/V" role (per E15) does not yet include per-app policy
- Persistence design implies durability expectations (E13 scaffold only, not real persistence)

### 3.4 Option C: Collar-Mediated Grants — STOP FIRST

**Architecture:**
```
Collar (future authority manager) mediates who may set policy.
SET_POLICY requires a Collar capability grant.
Policy objects are Collar-managed descriptors.
```

**STOP FIRST because:**
- Collar does not exist yet
- Entirely aspirational — no implementation timeline
- Would require new capability type, kernel mediation, and Collar server

---

## 4. Security Model

### 4.1 SUBSCRIBE Security

| Concern | Mitigation |
|---------|------------|
| Who may subscribe? | Same allowlist as LIST (PD 3, PD 6). Default-deny. |
| What data leaks? | Same aggregate lane counts as LIST — no sender identity, no content. |
| Subscription table | Bounded (4 entries max). No DoS via subscription flooding. |
| Lost wakeup | Generation model inherently handles this — next poll catches up. |
| Push flooding | Not applicable to Option A (polling). For push (future): rate-limit pushes to 1 per tick. |

### 4.2 SET_POLICY Security

| Concern | Mitigation |
|---------|------------|
| Who may set policy? | Only allowlisted admin PD (currently PD 3, silk-shell). Default-deny. |
| Policy scope | Per caller_pd only. Cannot escalate own privileges via policy. |
| Privacy override | Cannot reduce privacy level below what notify already enforces. Policy override is for INCREASING restriction (e.g., force an app to FullHidden). |
| Mute via policy | Same effect as OP_BELL_MUTE_SENDER. Policy mute is additive with MUTE_LIST. |

### 4.3 Privacy/Redaction Preservation

- SUBSCRIBE reply is same aggregate lane counts as LIST — no new disclosure
- SET_POLICY privacy override can only INCREASE restriction (never decrease)
- Policy cannot bypass redaction_class — structural metadata and secret content remain redacted
- Policy is applied at notify time (before enqueue), not at list time

### 4.4 Spam/Mute Interaction

```
NOTIFY → Mute check → Spam check → Policy override → Enqueue → (increment generation)
                                              ↑
                                       SET_POLICY can:
                                       - force lane to a low-urgency value
                                       - force FullHidden privacy
                                       - mute the sender entirely
```

- Policy mute is checked first (before spam budget), same as `is_muted()`
- Policy lane override happens after lane derivation — final lane is policy if set

---

## 5. Failure Modes and Premortem

### 5.1 Generation Counter Wraparound

| Scenario | Effect | Mitigation |
|----------|--------|------------|
| u64 counter wraps from MAX to 0 | Subscriber sees generation < cached → false positive change | O(1) false positive is fine — triggers one extra LIST poll. No correctness issue. |
| Counter not incremented on mute change | Subscriber misses mute update | Fix: always increment on any state change (enqueue, dismiss, clear, mute add/remove). |

### 5.2 Lost Wakeup (Polling Path — not applicable)

| Scenario | Effect | Mitigation |
|----------|--------|------------|
| Polling: subscriber misses an event between polls | Event visible on next poll | Acceptable latency bound = poll cadence (5s). |

### 5.3 Policy Persistence Drift

| Scenario | Effect | Mitigation |
|----------|--------|------------|
| Volatile: Bell restarts, policies lost | Apps revert to default privacy | Shell re-applies policies on boot (OPTION A only — acceptable for V1). |
| Sexstore-backed: sexstore down on boot | Bell blocks waiting for policy load | Bell falls back to no-policy (volatile-only) until sexstore responds. |

### 5.4 FullHidden Leak

| Scenario | Effect | Mitigation |
|----------|--------|------------|
| Policy override reduces privacy level | FullHidden events become visible | Enforced in SET_POLICY handler: privacy_override can only be ≥ current level. |
| Generation counter change reveals existence of event | Subscriber knows "something changed" but not what | Acceptable: generation is 1 bit of info. Aggregate counts already leak existence. |

### 5.5 Capability Bypass

| Scenario | Effect | Mitigation |
|----------|--------|------------|
| Rogue PD calls SUBSCRIBE | Rejected by server allowlist (same as LIST) | Default-deny. Only PD 3 and PD 6 allowlisted. |
| Rogue PD calls SET_POLICY | Rejected by server allowlist | Same allowlist, or restricted to PD 3 only. |

---

## 6. Recommended Implementation Sequence

### Phase E-1: Generation Counter Polling (NO KERNEL CHANGE)

**Files to change:** `servers/sexbell/src/main.rs`

**Changes:**
1. Add `static mut BELL_GENERATION: u64 = 0` incremented on every queue/mute change.
2. Add OP_BELL_SUBSCRIBE handler that returns current generation.
3. Optionally extend LIST reply to include generation (backward-compatible upper bits).
4. Reduce SilkBar poll cadence from 2s to 5s.

**Stop-first gates:**
- None — no kernel change, no ABI change, no persistence, no schema.

**Build:** Must pass. No new dependencies.

### Phase E-2a: Volatile SET_POLICY (NO STORAGE, NO KERNEL CHANGE)

**Files to change:** `servers/sexbell/src/main.rs`

**Changes:**
1. Add `static mut POLICY_TABLE: [PolicyEntry; 8]` with LRU eviction.
2. Add OP_BELL_SET_POLICY handler that writes policy table.
3. Modify NOTIFY handler to apply policy overrides before enqueue.

**Stop-first gates:**
- Do not add sexstore dependency.
- Do not add persistence.

### Phase E-2b: Sexstore-Backed Policy (STOP FIRST — requires schema gate)

**Prerequisites:**
1. Sexstore schema audit for Bell policy namespace.
2. Sexstore becomes available before Bell in boot order (currently Bell is last).
3. Persistent storage maturity beyond E13 RAM scaffold.

### Phase E-3: Kernel Push IPC (STOP FIRST — requires kernel ABI gate)

**Prerequisites:**
1. Kernel ABI design for inter-domain push notification.
2. Shared-memory or new syscall capability model.
3. Subscriber table in Bell with bounded capacity.

---

## 7. STOP FIRST Boundaries

| Component | Boundary | Condition | Phase |
|-----------|----------|-----------|-------|
| Kernel ABI | Push IPC, new syscall, shared memory | ⛔ STOP — no kernel changes in current roadmap | E-3 |
| Persistent schema | Sexstore policy namespace | ⛔ STOP — no storage schema design yet | E-2b |
| Collar authority | Collar-mediated policy grants | ⛔ STOP — Collar does not exist | Future |
| Server model | Blocking/async handler thread | ⛔ STOP — Bell is single-threaded by design | E-3 |

**Within bounds (safe to implement):**
| Component | Phase | Reason |
|-----------|-------|--------|
| Generation counter | E-1 | RAM only, no kernel, no ABI |
| Volatile policy table | E-2a | RAM only, bounded, no persistence |
| Allowlist expansion | E-1/E-2a | Existing pattern (BELL_LIST_ALLOWLIST) |

---

## 8. Summary of Options

| Feature | Option | Kernel Change? | Storage? | Recommendation |
|---------|--------|----------------|----------|---------------|
| SUBSCRIBE | A: Generation counter | ❌ No | ❌ No | ✅ **Phase E-1** |
| SUBSCRIBE | B: Kernel push IPC | ✅ Yes | ❌ No | ⛔ STOP FIRST (E-3) |
| SUBSCRIBE | C: One-shot wait/reply | ⛔ Blocking | ❌ No | ⛔ STOP FIRST |
| SET_POLICY | A: Volatile RAM table | ❌ No | ❌ No | ✅ **Phase E-2a** |
| SET_POLICY | B: Sexstore-backed | ❌ No | ✅ Yes | ⛔ STOP FIRST (E-2b) |
| SET_POLICY | C: Collar-mediated | ✅ Yes | ✅ Yes | ⛔ STOP FIRST (Future) |

---

## Appendix A: Data Structures (Conceptual)

```
// ── Generation counter ──
static mut BELL_GENERATION: u64 = 0;

// Increment in: notify accept, close, clear, mute_add, mute_remove
fn bump_generation() {
    unsafe { BELL_GENERATION = BELL_GENERATION.wrapping_add(1); }
}

// ── Policy table entry ──
#[repr(C)]
struct PolicyEntry {
    caller_pd:      u32,
    policy_mask:    u8,   // bit 0 = privacy_override valid, bit 1 = lane_override valid, bit 2 = force_mute
    privacy_level:  u8,   // 0..3 (only valid if policy_mask bit 0 set)
    lane_override:  u8,   // 0..5 (only valid if policy_mask bit 1 set)
}

const POLICY_TABLE_CAPACITY: usize = 8;
static mut POLICY_TABLE: [PolicyEntry; POLICY_TABLE_CAPACITY] = [...];
static mut POLICY_COUNT: usize = 0;

// ── Subscriber table (for future push IPC) ──
const SUBSCRIBER_CAPACITY: usize = 4;
struct SubscriberEntry {
    caller_pd: u32,
    last_generation: u64, // generation at last push
}
```

## Appendix B: Wire Format (Conceptual)

```
OP_BELL_SUBSCRIBE request (0xC5):
  arg0: u64 = 0 (reserved for future use, must be 0)
Reply:
  value: u64 = current generation counter
  (Caller compares with cached generation; if different, calls OP_BELL_LIST)

OP_BELL_SET_POLICY request (0xC6):
  arg0: u32 = target caller_pd (the app whose policy to override)
  arg1: u64 = policy_flags | (privacy_override << 8) | (lane_override << 16) | (mute << 24)
                bit 0 = apply privacy_override
                bit 1 = apply lane_override
                bit 2 = apply mute
                bits 8-9: privacy_override (0=Public .. 3=FullHidden)
                bits 16-18: lane_override (0=PASSIVE .. 5=SECURITY)
                bit 24: mute (0=unmute, 1=mute)
Reply:
  value: u64 = 0 (OK) or u64::MAX (error: invalid args, table full)
```

---

*End of BELL_PHASE_E_SUBSCRIBE_POLICY_DESIGN_V1.md*
