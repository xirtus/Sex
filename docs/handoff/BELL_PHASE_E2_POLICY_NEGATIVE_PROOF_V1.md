# BELL_PHASE_E2_POLICY_NEGATIVE_PROOF_V1

## Proof Method

Static code review + build proof.  No runtime synthetic test harness
exists for Bell (no way to craft arbitrary PDX calls from userspace
without a dedicated test PD).  All invariants are enforced at compile
time by the Rust type system and at runtime by the match-arm dispatch
and explicit guard checks.

---

## 1. Authority Proof

### Invariant: only PD 3 may call SET_POLICY

**Enforcement** (`servers/sexbell/src/main.rs`):

```rust
const BELL_POLICY_AUTHOR_ALLOWLIST: &[u32] = &[
    3,  // silk-shell (domain 3, policy owner)
];

fn is_policy_author_allowed(caller_pd: u32) -> bool {
    BELL_POLICY_AUTHOR_ALLOWLIST.contains(&caller_pd)
}
```

**SET_POLICY handler** (line ~1026):
```rust
if !is_policy_author_allowed(caller_pd) {
    // emit [bell.policy.deny] marker
    pdx_reply(caller_pd, u64::MAX);
    continue;
}
```

| Caller | Allowed | Reason |
|--------|---------|--------|
| PD 3 (silk-shell) | ✅ Yes | Listed in BELL_POLICY_AUTHOR_ALLOWLIST |
| PD 6 (silkbar) | ❌ Denied | Separate allowlist — excluded by design |
| Any other PD | ❌ Denied | Default-deny — not in allowlist |

**Note**: `BELL_POLICY_AUTHOR_ALLOWLIST` is a separate constant from
`BELL_LIST_ALLOWLIST`.  SilkBar is in `BELL_LIST_ALLOWLIST` (can LIST)
but NOT in `BELL_POLICY_AUTHOR_ALLOWLIST`.  Proof by construction:
both lists are static const slices; only PD 3 appears in the policy
list.

---

## 2. Privacy Invariant Proof

### Invariant A: SET_POLICY cannot reduce an existing override

**Enforcement** (SET_POLICY handler, line ~1097):
```rust
if active_flags & 1 != 0 {
    if let Some(existing) = find_policy(target_pd) {
        if existing.active_flags & 1 != 0 && privacy_val < existing.privacy_level {
            // emit [bell.policy.reject] reason=privacy_reduction
            pdx_reply(caller_pd, u64::MAX);
            continue;
        }
    }
}
```

| Scenario | Existing Override | New Override | Result |
|----------|------------------|-------------|--------|
| No existing policy | — | privacy=2 | ✅ Allowed (new entry) |
| Existing override | privacy=2 | privacy=2 | ✅ Allowed (no-op) |
| Existing override | privacy=2 | privacy=3 | ✅ Allowed (increase restriction) |
| Existing override | privacy=2 | privacy=1 | ❌ Rejected (would reduce) |
| Existing override | privacy=2 | privacy=0 | ❌ Rejected (would reduce to Public) |
| Existing entry, no privacy flag | flags=0x2 (lane only) | privacy=1 | ✅ Allowed (no existing privacy to reduce from) |

### Invariant B: effective_privacy = max(event_privacy, policy_privacy)

**Enforcement** (`apply_policy_privacy`, line ~318):
```rust
fn apply_policy_privacy(caller_pd: u32, event_privacy: u8) -> u8 {
    if let Some(entry) = find_policy(caller_pd) {
        if entry.active_flags & 1 != 0 {
            return core::cmp::max(event_privacy, entry.privacy_level);
        }
    }
    event_privacy
}
```

| Event Privacy | Policy Override | Effective Privacy | Reasoning |
|--------------|----------------|-------------------|-----------|
| 0 (Public) | — (no policy) | 0 | Unchanged |
| 0 (Public) | 0 (Public) | 0 | max(0,0) |
| 0 (Public) | 3 (FullHidden) | 3 | max(0,3) — policy increases |
| 3 (FullHidden) | 0 (Public) | 3 | max(3,0) — event privacy preserved |
| 2 (Restricted) | 1 (Sensitive) | 2 | max(2,1) — event wins |
| 1 (Sensitive) | 2 (Restricted) | 2 | max(1,2) — policy wins |

**Proof**: `core::cmp::max` guarantees the result is at least `event_privacy`.
Policy can never make an event MORE visible.  Event privacy is always
preserved as a floor.

---

## 3. Mute Proof

### Invariant: policy mute is additive with global mute list

**Enforcement** (NOTIFY handler, line ~470):
```rust
if is_muted(caller_pd) || is_policy_muted(caller_pd) {
    // emit [bell.notify.reject] reason=muted
    continue;
}
```

`is_muted()` checks the global `MUTE_LIST` array (managed by
`OP_BELL_MUTE_SENDER`).  `is_policy_muted()` checks the policy table
(`active_flags & (1<<2) && force_mute != 0`).

| MUTE_LIST | Policy Mute | NOTIFY Rejected? |
|-----------|------------|-----------------|
| not muted | no policy | ❌ No |
| muted | no policy | ✅ Yes (via is_muted) |
| not muted | force_mute=1 | ✅ Yes (via is_policy_muted) |
| muted | force_mute=1 | ✅ Yes (both, first wins) |

### Invariant: unsetting policy mute does NOT affect global mute list

**Proof by construction**: `is_muted()` and `is_policy_muted()` read
from independent data structures (`MUTE_LIST[]` vs `POLICY_TABLE[]`).
Removing a policy entry (calling SET_POLICY with `active_flags=0`)
only clears the policy table entry; it does not touch `MUTE_LIST`
(which only `add_mute` / `remove_mute` can modify).

**Policy mute bit fix**: `is_policy_muted` originally checked
`active_flags & 2` (bit 1, lane override) instead of
`active_flags & (1 << 2)` (bit 2, force_mute).  Fixed in commit
(before this handoff).

---

## 4. Generation Bump Proof

### Invariant A: add/modify/remove bumps

**Enforcement** (SET_POLICY handler, line ~1155):
```rust
if changed {
    bump_generation();
}
```

| Operation | `changed` | Bump? |
|-----------|-----------|-------|
| New entry added | true | ✅ Yes |
| Existing entry, field differs | true | ✅ Yes |
| Existing entry removed | true | ✅ Yes |
| No-op (identical values) | false | ❌ No |
| Rejected (authorization) | never reached | ❌ No |
| Rejected (invalid field) | never reached | ❌ No |
| Rejected (privacy reduction) | never reached | ❌ No |
| Rejected (table full) | never reached | ❌ No |

### Invariant B: rejected SET_POLICY does not bump

**Proof by control flow**: Every reject path contains `continue`,
which jumps to the top of the `loop` without executing
`bump_generation()` at line ~1155.  The four reject sites are:

1. Authorization deny (line ~1043): `pdx_reply(...); continue;`
2. Invalid field (line ~1064): `pdx_reply(...); continue;`
3. Privacy reduction (line ~1112): `pdx_reply(...); continue;`
4. Table full (line ~1148): `pdx_reply(...); continue;`

All four exit before the `if changed { bump_generation(); }` check.

### Invariant C: `changed` determined by field comparison

When an existing entry is found, `changed` is set by:
```rust
changed = old.active_flags != new_entry.active_flags
    || old.privacy_level != new_entry.privacy_level
    || old.lane_override != new_entry.lane_override
    || old.force_mute != new_entry.force_mute;
```

If all four fields are identical to the existing entry, `changed`
remains `false` (initialized to `false` at line ~1071) and
`bump_generation()` is not called.  This makes SET_POLICY idempotent.

For new entries (not found), `changed` is set to `true` (line ~1137).

For removal (all flags cleared), `changed` is set to `true` if an
entry was found and removed (line ~1091).

---

## 5. LIST/SUBSCRIBE Compatibility Proof

### Invariant: packed LIST format unchanged

**Proof by inspection**: The `OP_BELL_LIST` handler (line ~624 onwards)
was not modified by this change.  The packed reply format remains:
```
[63:56]=redacted [55:48]=lane5 [47:40]=lane4 [39:32]=lane3
[31:24]=lane2 [23:16]=lane1 [15:8]=lane0 [7:0]=total_visible
```

### Invariant: SUBSCRIBE generation still works

**Proof by inspection**: The `OP_BELL_SUBSCRIBE` handler (line ~1024)
was not modified.  It still replies with `BELL_GENERATION` and is
bumped on all state changes including policy table changes.

### Invariant: LIST privacy filter unchanged

LIST handler uses `caller_max_privacy` from `max_privacy_for_caller()`
which is independent of the policy table.  Policy overrides affect
what gets stored in the queue (via `effective_privacy`), not what
LIST reveals to each reader.  This is correct because policy is
applied at NOTIFY time, before enqueue — the queue entry already
contains the effective privacy level.

---

## 6. Test Result

```
./scripts/entrypoint_build.sh: PASS
```

Strings in sexbell binary confirmed:
```
[bell.policy.deny]
[bell.policy.set]
[bell.policy.reject] reason=invalid_field
[bell.policy.reject] reason=privacy_reduction
[bell.policy.reject] reason=table_full
```

No regression in existing strings:
```
[bell.subscribe.reply] gen=
[bell.subscribe.deny] caller_pd=
[bell.list.reply] total=
[bell.list.reject] reason=invalid_count caller_pd=
[bell.list.reject] reason=invalid_lane caller_pd=
```

---

## 7. Bug Fixed During Proof

**`is_policy_muted` bit check**: was `active_flags & 2` (bit 1,
lane_override), corrected to `active_flags & (1 << 2)` (bit 2,
force_mute).  Without this fix, policy mute would be triggered by the
lane_override flag instead of the force_mute flag, making lane
overrides unintentionally mute the target.

---

## Summary

| Invariant | Status | Method |
|-----------|--------|--------|
| PD 3 allowed, PD 6 denied, others denied | ✅ | Static list + guard |
| Policy cannot reduce existing override | ✅ | Explicit comparison + reject |
| effective_privacy = max(event, policy) | ✅ | `core::cmp::max` |
| Policy mute additive with global mute | ✅ | `||` in NOTIFY handler |
| Unsetting policy mute doesn't affect MUTE_LIST | ✅ | Independent data structures |
| add/modify/remove bumps generation | ✅ | `changed` flag |
| Idempotent SET_POLICY does not bump | ✅ | Field comparison |
| Rejected SET_POLICY does not bump | ✅ | `continue` before bump |
| LIST packed format unchanged | ✅ | No code touched |
| SUBSCRIBE generation still works | ✅ | No code touched |
| `is_policy_muted` bit value correct | ✅ | Fixed during proof |
| Build passes | ✅ | `entrypoint_build.sh` |
