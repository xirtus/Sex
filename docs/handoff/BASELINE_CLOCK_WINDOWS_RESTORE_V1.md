# BASELINE_CLOCK_WINDOWS_RESTORE_V1

## Baseline Restore Status
- Baseline restored.
- SilkBar clock works.
- Tiled windows work.

## Quil Regression Status
- Broken Quil proof/input/autosave/F9 path is reverted/absent.
- `docs/handoff/QUIL_MVP_V1.md` absence is acceptable at this restore point.

## Commit Separation Rule
- APIC ISO fix was committed separately as kernel-only work.
- Baseline recovery handoff is docs-only and must not mix runtime/kernel changes.

## Guardrails For Next Quil Work
Until static text proof passes, Quil prompts must not touch:
- `servers/sexinput/*`
- `servers/silk-shell/*` lifecycle/F9 paths
- SilkBar/clock paths
- `kernel/*`
- sex-pdx ABI
- SexFiles save/restore flows

## Next Safe Task
- `QUIL_STATIC_TEXT_ONLY_V1`
