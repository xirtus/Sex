# SPINDLE_NOTIFY_ALIAS_AUDIT_V1

## Scope
- Audit alias behavior for daily-driver command shortcuts in `apps/spindle`.
- No behavior change required.

## Findings
- Alias markers already present and stable:
  - `d -> daily`
  - `b -> blockers`
  - `k -> keys`
  - `a -> apps`
  - `q -> status`
  - `n -> notify`
- Marker contract present:
  - `[spindle.alias.exec] alias=NAME target=NAME ok=1`
- `n <msg>` maps to `notify <msg>` through existing parser and command dispatch.

## Evidence
- Alias remap path in `dispatch()` at `apps/spindle/src/main.rs`.
- Proof hook emits `[spindle.alias.proof.done]` in existing proof flow.

## Verdict
- PASS (no source changes needed).
