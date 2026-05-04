# PHASE 00: Baseline Proof + Anti-Drift Gates

## Goal
Stop losing time to false "done." Preserve and gate the current working baseline before adding ambition. Produce a repeatable CI pipeline that catches regressions before they land.

## Ownership
- **Gate definitions**: shared (kernel + servers + build system)
- **Enforcement**: shell scripts + CI-ready
- **Process owner**: developer (you, running the gates)

## What Already Exists
- Build gate: `./scripts/entrypoint_build.sh` works
- Boot gate: QEMU boots all 6 PDs, clock ticks, scheduler survives
- ABI hash: `sexos_build_spec.toml` tracks hashes
- No formal gate runner script — gates are manual grep checks

## Bundle

| Task | Scope | Verification | Effort |
|------|-------|-------------|--------|
| Canonical handoff truth | `docs/handoff/` | All handoffs reference committed state | 1h |
| Build gate script | `scripts/gate_build.sh` | Runs default + synthetic, exits nonzero on failure | 30min |
| Boot gate script | `scripts/gate_boot.sh` | QEMU boot, greps for all PDs online | 1h |
| Runtime marker gate | `scripts/gate_markers.sh` | All mandatory proof markers fire | 30min |
| Fault scan gate | integrated into boot gate | grep for panic/#PF/#GP, zero faults | 10min |
| no_std/POSIX/std/thread gate | `scripts/gate_no_std.sh` | Zero POSIX/std/thread imports in kernel/servers | 20min |
| Forbidden framebuffer writer gate | `scripts/gate_fb.sh` | Only sexdisplay writes to framebuffer | 15min |
| ABI hash guard | build script | Hash mismatch = build abort | built-in |
| **All gates in one runner** | `scripts/gate_all.sh` | Single `./gate_all.sh` → pass/fail | 30min |

## Smallest First Step
Create `scripts/gate_build.sh` that wraps `entrypoint_build.sh` and checks return code. That alone catches the most common regression (accidental compile break). Ship it before any other gate.

## Dependencies
- **Blocking**: None (this is the first phase)
- **Blocked by**: Nothing
- **Can parallelize with**: Phase 1 and Phase 2 (gates are independent of feature work)

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Gate scripts are too strict (false positives) | Medium | High (blocks all work) | Start lenient, tighten gradually. Gate_all.sh should have a `--strict` flag for CI vs `--lenient` for dev. |
| Gate scripts don't run in CI (no CI) | High | Low (manual gating still works) | Document that gates are manual until CI is set up. Don't block work on CI infrastructure. |
| Handoff doc rot (docs reference stale state) | Medium | Medium | Gate markers check at runtime, not document timestamps. Docs are secondary. |

## Exit Criteria (Done Checklist)
- [ ] `scripts/gate_build.sh` exists and passes on current master
- [ ] `scripts/gate_boot.sh` boots QEMU and confirms all 6 PDs
- [ ] `scripts/gate_markers.sh` confirms all proof markers
- [ ] `scripts/gate_no_std.sh` returns zero POSIX/std/thread hits
- [ ] `scripts/gate_fb.sh` confirms only sexdisplay framebuffer writes
- [ ] `scripts/gate_all.sh` runs all gates and exits 0
- [ ] All gate scripts checked into git
- [ ] README documents how to run gates

## Testing Strategy
- Gates are self-testing (exit 0 = pass, exit nonzero = fail)
- Test each gate script independently before adding to runner
- `gate_all.sh` tests that all pass together (catches cross-gate issues)

## Efficiency Opportunity
**Phase 0 runs continuously** — gates aren't a one-time deliverable. Create the scripts but keep them live: every PR/commit should re-run `gate_all.sh`. This is the highest-leverage investment in the entire 12-phase plan. Skipping gate automation guarantees repeat failures.

## Completeness Gain
**+5% reliability** (not visible features). Prevents regression drift. Multiplier effect: every subsequent phase is faster because regressions are caught immediately.

## Files Changed
- `scripts/gate_build.sh` (new)
- `scripts/gate_boot.sh` (new)
- `scripts/gate_markers.sh` (new)
- `scripts/gate_no_std.sh` (new)
- `scripts/gate_fb.sh` (new)
- `scripts/gate_all.sh` (new)
- `README.md` (gate documentation)

## Forbidden
- Feature work
- Renderer changes
- Protocol changes
- Kernel edits

## Next Phase
PHASE_01_SILK_DISPLAY_CONTRACT_RENDER.md

## Parallel Note
Phase 0, 1, and 2 can proceed in parallel — they touch different ownership domains (gates vs sexdisplay vs silk-shell).
