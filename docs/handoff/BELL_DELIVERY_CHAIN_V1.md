# BELL_DELIVERY_CHAIN_V1

## Route Chosen
Reused existing route (no ABI/opcode changes):
1. Sender calls `OP_BELL_NOTIFY` to `sexbell`.
2. `sexbell` validates bounded event fields and sender policy/mute/spam checks.
3. Accepted events are queued and generation bumped.
4. `silkbar` polls Bell via `OP_BELL_SUBSCRIBE`; on generation change it calls `OP_BELL_LIST`.
5. `silkbar` forwards compact Bell presence/count state to `sexdisplay` via `OP_SILKBAR_UPDATE::SetBellPresence`.

## Proof Gate
- `SEXOS_BELL_DELIVERY_PROOF=1`

## Proof Markers
- `sexbell`:
  - `[bell.event.accept]`
  - `[bell.event.reject]`
- `silkbar`:
  - `[bell.poll.ok]`
  - `[silkbar.bell.state]`

## Notes
- No popup behavior added.
- No renderer policy added.
- No persistence added.
- No kernel or `sex-pdx` ABI edits.

## Remaining Bell Risks
- Delivery is poll-based (`SUBSCRIBE`+`LIST` cadence), not push/interrupt delivery.
- LIST payload is aggregate/compact; no detailed bounded event text transport in this chain.
- Allowlist/cap policy is static; capability policy evolution is still pending.
