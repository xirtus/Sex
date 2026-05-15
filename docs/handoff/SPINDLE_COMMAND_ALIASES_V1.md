# SPINDLE_COMMAND_ALIASES_V1

Status: implemented

Aliases added:
- d -> daily
- b -> blockers
- k -> keys
- a -> apps
- q -> status
- n <msg> -> notify <msg>

Markers:
- [spindle.alias.exec] alias=NAME target=NAME ok=N
- [spindle.alias.proof.done] ok=N

Safety:
- No storage semantics changed.
- `q` maps to status (safe, non-exit behavior).
