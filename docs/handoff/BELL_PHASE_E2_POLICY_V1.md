# BELL_PHASE_E2_POLICY_V1

## Status: IMPLEMENTED

## Summary

Volatile RAM-only Bell policy table (`OP_BELL_SET_POLICY`).  No
persistence, no kernel change, no sexstore, no Collar.

## Opcode

`OP_BELL_SET_POLICY = 0xC6` (pre-existing in `sex-pdx`).

## Authority

Only PD 3 (silk-shell) may call SET_POLICY.  Enforced by
`BELL_POLICY_AUTHOR_ALLOWLIST` — a separate allowlist from
`BELL_LIST_ALLOWLIST`.  SilkBar (PD 6) is explicitly excluded.

Default-deny: any unauthorized caller receives `u64::MAX` reply and a
`[bell.policy.deny]` marker.

## Payload Encoding

```
OP_BELL_SET_POLICY (0xC6):
  arg0 = target_pd (the app whose policy to override)
  arg1 = packed policy:
    bit 0     = privacy_override active
    bit 1     = lane_override active
    bit 2     = force_mute active
    bits 8-9  = privacy_override value (0=Public .. 3=FullHidden)
    bits 16-18 = lane_override value (0=PASSIVE .. 5=SECURITY)
    bit 24    = force_mute value (0=unmuted, 1=muted)

Reply: 0 (OK), u64::MAX (error)
```

## Policy Table Shape

```rust
const POLICY_TABLE_CAPACITY: usize = 8;

struct PolicyEntry {
    target_pd:     u32,   // 0 = unused slot
    active_flags:  u8,    // bit 0=privacy, bit 1=lane, bit 2=mute
    privacy_level: u8,    // 0..3
    lane_override: u8,    // 0..5
    force_mute:    u8,    // 0/1
}
```

- 8-entry fixed array, linear scan (small enough).
- `target_pd == 0` marks unused slot.
- Entries removed by calling SET_POLICY with `active_flags == 0`.
- Table full returns `u64::MAX` (caller should remove an entry first).

## Privacy Invariant

1. **SET_POLICY**: new `privacy_override` must be ≥ existing override
   (if any).  Cannot reduce restriction for a target that already has
   a privacy policy.  Rejected with `reason=privacy_reduction`.

2. **NOTIFY**: effective privacy = `max(event_privacy, policy_privacy)`.
   Policy can only increase restriction, never decrease it.  If event
   is FullHidden (3) and policy says Public (0), effective = 3 (event
   wins).

3. **Lane override**: replaces derived lane entirely if policy has
   `lane_override` active.

4. **Mute via policy**: checked alongside `is_muted()` at NOTIFY time.
   `is_policy_muted(target_pd)` returns true if policy has
   `force_mute=1` active.

## Generation Bump

Generation is bumped only when the policy table actually changes:
- New entry added (target_pd not previously policied)
- Existing entry modified (any field differs)
- Entry removed (all flags cleared)

No bump if SET_POLICY is called with identical values (idempotent).

## Markers

| Marker | Budget | Meaning |
|--------|--------|---------|
| `[bell.policy.set]` | 8 | Policy set/updated/removed |
| `[bell.policy.deny]` | 8 | Unauthorized caller |
| `[bell.policy.reject] reason=invalid_field` | 8 | Invalid field value |
| `[bell.policy.reject] reason=privacy_reduction` | 8 | Would reduce restriction |
| `[bell.policy.reject] reason=table_full` | 8 | 8 entries exhausted |

## NOTIFY Integration

Policy is applied at NOTIFY time, after lane derivation and spam check
but before enqueue:

1. Policy mute check alongside `is_muted()` (reject before any processing)
2. `effective_privacy = max(privacy_level, policy_privacy_override)`
   applied to the value stored in the queue entry
3. `effective_lane = policy_lane_override` if active, else derived lane

## Files Changed

| File | Change |
|------|--------|
| `servers/sexbell/src/main.rs` | Add policy table, SET_POLICY handler, NOTIFY policy integration |
| `docs/handoff/BELL_PHASE_E2_POLICY_V1.md` | Created |

## Build

`./scripts/entrypoint_build.sh` passes.

## Next Recommended Phase

1. **BELL_PHASE_E3_PUSH**: Kernel push IPC for true zero-poll
   notification (requires kernel ABI change — STOP FIRST).
2. **BELL_PHASE_F_ACTION_DISPATCH**: Route action callbacks to
   registered action handlers.
3. **SEXSTORE_BELL_POLICY_PERSISTENCE**: Back policy table to sexstore
   so policies survive Bell restart (requires sexstore schema gate).
