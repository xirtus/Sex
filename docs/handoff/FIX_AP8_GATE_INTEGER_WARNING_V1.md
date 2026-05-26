# FIX_AP8_GATE_INTEGER_WARNING_V1

## Scope
- Mission: remove `integer expected` warning from `scripts/daily_driver_master_gate.sh` during AP8-related ordering checks.
- Constraints honored:
  - no runtime code changes
  - patched only `scripts/daily_driver_master_gate.sh`
  - no broad refactor
  - AP8 gate semantics preserved
  - default SKIP behavior unchanged

## Backup Before Changes
- Created backup:
  - `scripts/daily_driver_master_gate.sh.bak.20260526-233353`

## Exact Fix

### 1) AP8 click focus edge ordering sanitization
In `click_focus_button_edges` gate block, replaced raw line-number use with numeric sanitization:
- before: used `down_line` / `up_line` directly from `grep -n ... | cut -d: -f1`
- after:
  - capture raw values as `down_line_raw` / `up_line_raw`
  - sanitize to digits only:
    - `down_line="$(printf '%s' "${down_line_raw:-}" | tr -cd '0-9')"`
    - `up_line="$(printf '%s' "${up_line_raw:-}" | tr -cd '0-9')"`
  - preserve fail-safe semantics:
    - `has_down` / `has_up` are `1` only when sanitized numeric strings are non-empty
    - PASS only when both exist and `down_line < up_line`
    - otherwise FAIL when begin marker is present

### 2) Removed warning source at line 4953
The warning was emitted by a different block (`linen_sexobject_native_persist`), not AP8.
- before: inline `-ge 1` comparisons on values that could include quotes/non-numeric content
- after: each value in `sf_stage_detail` is sanitized with `tr -cd '0-9'` and checked via string compare (`!= "0"`), eliminating integer parsing warnings without changing gate outcome logic.

## Verification
Command run:
```bash
./scripts/daily_driver_master_gate.sh /tmp/click_focus_drag_impl_v1_rerun.log | rg "click_focus_button_edges|FINAL:|integer expected"
```

Output:
```text
  click_focus_button_edges     PASS   begin+down+up present and ordered
  FINAL: PASS (285 gates proved, 116 skipped, 0 faults)
```

No `integer expected` lines remained.
