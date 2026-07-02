# SPINDLE_SEARCH_HELP_V1 — Handoff

## Goal
Add `search` command to Spindle explaining Quil find (V10 local), Linen search
status (ABI-blocked), and available search paths.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `apps/spindle/src/main.rs` | `search` dispatch arm, search help proof gate | +20 |

## Command
| Command | Behaviour | ok |
|---------|-----------|----|
| `search` | Renders search overview: Quil find, Linen ABI blocker, files ls filter | 1 |

## Markers
```
[spindle.search.help] section=search ok=N reason=...
[spindle.search.help.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_SPINDLE_SEARCH_HELP_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- Gate `spindle_search_help`: PASS

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No cross-PD queries — static help only
