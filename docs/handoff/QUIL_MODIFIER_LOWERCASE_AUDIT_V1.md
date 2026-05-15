# QUIL_MODIFIER_LOWERCASE_AUDIT_V1

## Goal
Audit and implement shift modifier tracking + lowercase mapping in Quil.

## Fix
- Added `SHIFT_HELD` static bool, tracked via scancode 0x2A press / 0xAA release
- Updated `scancode_to_char(scancode, shift)` — lowercase when shift off, uppercase when on
- 26 letters + 10 digit/symbol pairs now shift-aware
- Shift key intercepted before palette/text dispatch (modifier, not character)

## Proof
- Audit: `[quil.mod.audit] has_mod=1` — shift tracked via scancode 0x2A
- Mapping: 'a' (0x61) no shift, 'A' (0x41) with shift

## Result
53/53 PASS. All 4 new V11 gates pass.

## Safety
No kernel/ABI/USB/pointer changes. Bounded static bool. Scancode set 1 only.
