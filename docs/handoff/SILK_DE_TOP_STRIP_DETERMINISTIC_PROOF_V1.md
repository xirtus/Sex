# SILK_DE_TOP_STRIP_DETERMINISTIC_PROOF_V1

Date: 2026-05-22
Baseline commit: `d0b1296e`

## Files changed
- `servers/sexdisplay/src/main.rs`
- `scripts/daily_driver_master_gate.sh`
- `docs/handoff/SILK_DE_TOP_STRIP_DETERMINISTIC_PROOF_V1.md`

Backups created:
- `crates/silkbar-model/src/lib.rs.top_strip_deterministic_v1.bak`
- `servers/sexdisplay/src/main.rs.top_strip_deterministic_v1.bak`
- `scripts/daily_driver_master_gate.sh.top_strip_deterministic_v1.bak`

## Proof design
Deterministic top-strip proof is implemented in `sexdisplay` as a one-shot offscreen render/hash lane:
- fixed offscreen buffer dimensions: `1280x51`
- deterministic model vector applied to a local `SilkBar` copy
- same top-strip render primitives used by runtime (`bar_color`, `clock_fg_at`, `bell_badge_at`, `DEFAULT_THEME.panel_glow`)
- every write into the proof buffer is indexed and bounds-checked (`idx < PROOF_PIXELS`)
- no physical framebuffer writes are performed by this proof lane

This keeps `sexdisplay` as sole renderer and avoids policy ownership changes.

## Deterministic vector contents
Applied in order:
1. `SetWorkspaceActive idx=2 active=0`
2. `SetWorkspaceActive idx=1 active=1`
3. `SetWorkspaceUrgent idx=4 urgent=1`
4. `SetChipVisible idx=2 visible=1`
5. `SetChipKind idx=2 kind=Battery`
6. `SetClock hh=10 mm=27 ss=42`

Theme token usage is explicitly logged via:
- `panel_fill`
- `panel_glow`
- `active`
- `urgent`
- `text`

## Hash algorithm
FNV-1a 64-bit:
- offset basis: `0xcbf29ce484222325`
- prime: `0x100000001b3`
- iterate pixels in deterministic row-major order
- each `u32` pixel hashed byte-by-byte in little-endian order (`b0,b1,b2,b3`)

## Expected hash
- observed hash: `0x9B5D54E17BDFA6F1`
- expected constant locked to: `0x9B5D54E17BDFA6F1`

## Proof markers
- `[silk.de.topstrip.proof.begin] w=... h=...`
- `[silk.de.topstrip.proof.vector] ...`
- `[silk.de.topstrip.proof.theme] ...`
- `[silk.de.topstrip.proof.hash] hash=0x...`
- `[silk.de.topstrip.proof.pass] hash=0x...`
- `[silk.de.topstrip.proof.fail] expected=... got=...`

## Gate
New/updated gate behavior in `daily_driver_master_gate.sh`:
- Gate name: `silk_de_topstrip_deterministic`
- PASS: `[silk.de.topstrip.proof.pass]` present and no fault marker path hit
- FAIL: `[silk.de.topstrip.proof.fail]` present
- FAIL: `#PF/#GP/panic/KERNEL PANIC/fault.kill.*(silkbar|sexdisplay)` present in proof-enabled lane
- SKIP: no explicit `[silk.de.topstrip.proof.begin]`

Legacy `top_strip_hash` false-fail risk removed by mirroring this explicit sentinel logic (no strict fail without explicit proof begin).

## Proof commands
- `./scripts/entrypoint_build.sh`
- `./scripts/run_daily_driver_proof.sh /tmp/silk_de_topstrip_deterministic_v1_strict.log`
- `./scripts/daily_driver_master_gate.sh /tmp/silk_de_topstrip_deterministic_v1_strict.log | tee /tmp/silk_de_topstrip_deterministic_v1_strict_gate.txt`
- `rg -n "silk.de.contract|silk.de.topstrip|top_strip_hash|silk_de_topstrip_deterministic|FINAL:|FAIL|#PF|#GP|panic|KERNEL PANIC|fault.kill" /tmp/silk_de_topstrip_deterministic_v1_strict.log /tmp/silk_de_topstrip_deterministic_v1_strict_gate.txt`

## Gate result
- `silk_de_contract_lock PASS`
- `silk_de_topstrip_deterministic PASS`
- `top_strip_hash` not false-failing
- `FINAL: PASS (258 gates proved, 88 skipped, 0 faults)`

## Fault scan
From proof log and gate output:
- no `#PF`
- no `#GP`
- no `panic`
- no `KERNEL PANIC`
- no `fault.kill` for `silkbar`/`sexdisplay`

## What remains for Silk DE 100
1. renderer conformance audit/final cleanup
2. integrated interaction scenario proof
3. Frame Lights explicit proof sentinel/implementation if needed
4. safe glass color polish
5. final Silk DE 100 release handoff/tag
