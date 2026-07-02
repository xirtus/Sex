# SLOT2_STOP_FIRST_GATECHECK_V3

## Scope
Docs-only gatecheck before any slot2 runtime implementation change.

## Required Preconditions
1. Slot1 baseline markers confirmed on latest SHA.
2. Slot2 ownership/context marker audit refreshed.
3. Clear missing-marker chain documented.

## STOP FIRST Approval Required For
- kernel interrupt or dispatch edits
- `sex-pdx`/ABI contract changes
- `sexusb` behavior changes
- `sexinput` pointer route changes

## Allowed Without STOP FIRST
- handoff docs
- marker naming docs
- non-behavioral grep/reporting scripts

## Next Candidate Missions (post-approval)
1. slot2 role classification marker parity
2. slot2 context readiness proof lane
3. slot2 behavior patch set (bounded, staged)
