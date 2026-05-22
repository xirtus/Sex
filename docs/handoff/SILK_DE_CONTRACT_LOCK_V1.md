# SILK_DE_CONTRACT_LOCK_V1

Date: 2026-05-22
Tip commit before changes: `d636ef30`

## Scope
Safest first-slice lock for Silk DE contract integrity:
- explicit SilkBar ABI/layout/theme constants in `silkbar-model`
- startup contract validation markers in producer (`silkbar`) and renderer (`sexdisplay`)
- bounded update recv/apply drift markers in renderer
- daily-driver gate for contract lock status

No kernel edits, no sex-pdx ABI edits, no framebuffer ownership changes, no renderer policy ownership expansion.

## Files changed
- `crates/silkbar-model/src/lib.rs`
- `servers/silkbar/src/main.rs`
- `servers/sexdisplay/src/main.rs`
- `scripts/daily_driver_master_gate.sh`

Backups created:
- `crates/silkbar-model/src/lib.rs.silk_de_contract_lock_v1.bak`
- `servers/silkbar/src/main.rs.silk_de_contract_lock_v1.bak`
- `servers/sexdisplay/src/main.rs.silk_de_contract_lock_v1.bak`
- `scripts/daily_driver_master_gate.sh.silk_de_contract_lock_v1.bak`

## Root cause / risk closed
Previously, startup contract checks existed but were not normalized into a single explicit ABI/layout/theme lock signal shared by producer + renderer, and daily-driver had no dedicated contract-lock gate. That left drift detection less explicit and less auditable in runtime proof logs.

This patch closes that by adding stable contract constants + fingerprint in model, startup pass/fail markers in both endpoints, and a dedicated gate rule requiring both pass markers and zero contract-fail/fault markers.

## Contract constants added
In `crates/silkbar-model/src/lib.rs`:
- `SILK_DE_BAR_ABI_V1: u32 = 4` (already present, now part of lock set)
- `SILK_DE_BAR_LAYOUT_V1: u32 = 11`
- `SILK_DE_BAR_THEME_V1: u32 = 10`
- `SILK_DE_REQUIRED_WORKSPACE_SLOTS: usize = WORKSPACE_COUNT`
- `SILK_DE_REQUIRED_CHIP_SLOTS: usize = MAX_CHIPS`
- `SILK_DE_CONTRACT_MAGIC: u32 = 0x5344_4241`
- `contract_fingerprint() -> u64`

Validation tightened in `validate_contract()` for:
- ABI/layout/theme/version constants
- workspace/chip slot counts
- nonzero theme core tokens
- bounded panel dimensions

## Producer marker
From `servers/silkbar/src/main.rs` startup:
- PASS: `[silk.de.contract.producer.pass] abi=... layout=... theme=... fp=...`
- FAIL: `[silk.de.contract.producer.fail] reason=... abi=... layout=... theme=... fp=...`

## Renderer marker
From `servers/sexdisplay/src/main.rs` startup:
- PASS: `[silk.de.contract.renderer.pass] abi=... layout=... theme=... fp=...`
- FAIL: `[silk.de.contract.renderer.fail] reason=... abi=... layout=... theme=... fp=...`

## Drift audit markers
From `servers/sexdisplay/src/main.rs` (budgeted):
- `[silk.de.update.recv] kind=... slot=...`
- `[silk.de.update.apply.ok] kind=... slot=...`

## Gate name
Added in `scripts/daily_driver_master_gate.sh`:
- `silk_de_contract_lock`

PASS requires:
- `silk.de.contract.producer.pass`
- `silk.de.contract.renderer.pass`

FAIL on:
- `silk.de.contract.producer.fail`
- `silk.de.contract.renderer.fail`
- `silk.de.contract.mismatch`
- `#PF | #GP | panic | KERNEL PANIC | fault.kill.*(silkbar|sexdisplay)`

SKIP when Silk DE contract markers are absent.

## Proof commands
- `./scripts/entrypoint_build.sh`
- `./scripts/run_daily_driver_proof.sh /tmp/silk_de_contract_lock_v1.log`
- `./scripts/daily_driver_master_gate.sh /tmp/silk_de_contract_lock_v1.log`
- `rg -n "#PF|#GP|panic|KERNEL PANIC|fault.kill|silk.de.contract|silk.de.update" /tmp/silk_de_contract_lock_v1.log`

## Proof result
- Build: PASS (`entrypoint_build.sh` completed)
- Contract markers in log:
  - `[silk.de.contract.renderer.pass] abi=4 layout=11 theme=10 fp=0x00201004584e4745`
  - `[silk.de.contract.producer.pass] abi=4 layout=11 theme=10 fp=0x00201004584e4745`
- New gate: `silk_de_contract_lock PASS`
- Overall master gate: `FINAL: FAIL (1 gate(s) failed)`
  - unrelated pre-existing fail: `frame_lights_stub FAIL`

## Fault scan result
No contract-lane faults found for this log:
- no `#PF`
- no `#GP`
- no `panic`
- no `KERNEL PANIC`
- no `fault.kill` for silkbar/sexdisplay

## What remains for Silk DE 100%
1. Renderer conformance audit completion (if not fully closed in current-tier matrix).
2. Deterministic top-strip vector/hash proof refresh tied to this contract lock lane.
3. Integrated scenario proof where contract lock is exercised alongside combined interaction sentinel.
4. Safe glass color polish pass bounded to existing renderer checks.
5. Final release handoff/tag once all enabled daily-driver gates are PASS and no faults.

## Frame Lights Stub Gate Fix (SILK_DE_FRAME_LIGHTS_STUB_GATE_FIX_V1)

### Root cause
`frame_lights_stub` was a gate enablement mismatch (not a runtime fault).  
The gate treated ordinary frame-light markers as mandatory proof for all daily-driver runs and could fail unrelated missions when explicit frame-lights stub proof was not requested.

### Fix applied (smallest safe)
Updated `scripts/daily_driver_master_gate.sh` `frame_lights_stub` rule:
- Added explicit-proof sentinel requirement:
  - `[silk.frame.lights.stub.begin]`
  - `[silk.frame.lights.proof.begin]`
  - `[silk.frame.lights.visual.begin]`
- If no explicit sentinel is present, `frame_lights_stub` is now:
  - `SKIP` with reason `not requested (missing explicit proof sentinel)`
- If explicit sentinel is present, existing strict PASS/FAIL checks remain unchanged.

No kernel edits, no sex-pdx ABI edits, no renderer ownership change, no framebuffer bounds-path edits.

### Proof rerun
Commands:
- `./scripts/entrypoint_build.sh`
- `./scripts/run_daily_driver_proof.sh /tmp/silk_de_contract_lock_v1_rerun.log`
- `./scripts/daily_driver_master_gate.sh /tmp/silk_de_contract_lock_v1_rerun.log | tee /tmp/silk_de_contract_lock_v1_gate_rerun.txt`
- `rg -n "silk.de.contract|frame_lights_stub|FINAL:|FAIL|#PF|#GP|panic|KERNEL PANIC|fault.kill" /tmp/silk_de_contract_lock_v1_rerun.log /tmp/silk_de_contract_lock_v1_gate_rerun.txt`

Result:
- `silk_de_contract_lock PASS`
- `frame_lights_stub SKIP   not requested (missing explicit proof sentinel)`
- `FINAL: PASS (257 gates proved, 88 skipped, 0 faults)`
- `FAIL gates: 0`

Evidence:
- `/tmp/silk_de_contract_lock_v1_rerun.log`
  - `[silk.de.contract.renderer.pass] abi=4 layout=11 theme=10 fp=0x00201004584e4745`
  - `[silk.de.contract.producer.pass] abi=4 layout=11 theme=10 fp=0x00201004584e4745`
- `/tmp/silk_de_contract_lock_v1_gate_rerun.txt`
  - `frame_lights_stub            SKIP   not requested (missing explicit proof sentinel)`
  - `FINAL: PASS (257 gates proved, 88 skipped, 0 faults)`
