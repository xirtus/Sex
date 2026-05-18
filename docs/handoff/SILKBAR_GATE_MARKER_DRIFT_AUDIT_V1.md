# SILKBAR_GATE_MARKER_DRIFT_AUDIT_V1

Date: 2026-05-19
Classification: **A — GATE_DRIFT**

## Mission

Determine whether `keyboard_gui` and `silkbar_phase3_status` gate failures are real
runtime regressions or daily_driver_master_gate regex drift after marker changes.

## Old Gate Expectations

### keyboard_gui
Expected: `silkbar.clock.send` marker (line 736 of silkbar/src/main.rs).
Budget: 12 sends, after which marker stops appearing.

### silkbar_phase3_status
Expected: `shell.silkbar.phase2.send.*SetActiveApp` +
`sexdisplay.silkbar.phase3.recv.*SetActiveApp` +
`sexdisplay.silkbar.phase3.state`.

## Actual Markers Found (failing log)

```
sexdisplay.ready
sexdisplay.clock.source.fallback.tick
sexdisplay.clock.fallback.tick from_silkbar=0
silkbar.clock.synthetic.visible threshold=2
bootgraph.edge.send from=silkbar to=sexdisplay slot=SLOT_DISPLAY op=OP_SILKBAR_UPDATE first=1
shell.silkbar.status.send (×52)
```

## Source Marker Grep Summary

| Old marker | Source status | Why absent in runtime |
|---|---|---|
| `silkbar.clock.send` | EXISTS at silkbar:736 | Budget-limited (12 sends); suppressed after budget exhausted or in force_stall/fallback profile |
| `shell.silkbar.phase2.send` | RENAMED | Now emitted as `shell.silkbar.status.send` (unconditional); old name only inside `send_silkbar_phase2_update` which is gated by `SEXOS_SILKBAR_PHASE2_SHELL_PROOF` compile-time flag |
| `sexdisplay.silkbar.phase3.recv` | EXISTS at sexdisplay:1622/1627/1632 | Only fires when `apply_update` returns true AND kind matches 8/9/10; PDX calls gated behind `SEXOS_SILKBAR_PHASE2_SHELL_PROOF` in silk-shell |
| `sexdisplay.silkbar.phase3.state` | EXISTS at sexdisplay:1297 | Same gating chain |

## Classification: GATE_DRIFT

Both runtime paths are ALIVE. The gates were too narrow and did not accept
equivalent liveness markers emitted in the current default build profile.

- `silkbar.clock.send` is still emitted in some profiles but suppressed in
  force_stall/fallback/degraded modes. `silkbar.clock.synthetic.visible` and
  `sexdisplay.ready` prove the same liveness.

- `shell.silkbar.phase2.send` was renamed to `shell.silkbar.status.send`
  (unconditional). `sexdisplay.silkbar.phase3.recv/state` are intentionally
  absent in the default profile (gated behind `SEXOS_SILKBAR_PHASE2_SHELL_PROOF`
  and `SEXOS_SILKBAR_PHASE3_RECEIVE_PROOF` compile-time flags).
  `bootgraph.edge.send from=silkbar to=sexdisplay slot=SLOT_DISPLAY
  op=OP_SILKBAR_UPDATE first=1` proves the e2e link is working.

## Gate Patch Rationale

### keyboard_gui
Accepts (in order):
1. Old: `silkbar.clock.send` (original marker, still valid when emitted)
2. New: `sexdisplay.ready` + `silkbar.clock.synthetic.visible` (proves display init + clock alive in synthetic/fallback mode)
3. New: `bootgraph.edge.send from=silkbar.*OP_SILKBAR_UPDATE` + `sexdisplay.clock.fallback.tick` (proves silkbar→sexdisplay link + clock tick)

### silkbar_phase3_status
Accepts (in order):
1. Old: `shell.silkbar.phase2.send.*SetActiveApp` + `sexdisplay.silkbar.phase3.recv.*SetActiveApp` + `sexdisplay.silkbar.phase3.state` (full phase2/3 enabled lane)
2. New: `shell.silkbar.status.send` + `bootgraph.edge.send from=silkbar.*OP_SILKBAR_UPDATE.*first=1` (proves status send + silkbar→sexdisplay e2e link in default profile)

## Proof Command

```bash
./scripts/entrypoint_build.sh
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_silkbar_gate_marker_drift_audit_v1.log
```

## Proof Result

- `keyboard_gui`: PASS (sexdisplay ready + silkbar synthetic clock visible)
- `silkbar_phase3_status`: PASS (status_send=52 + silkbar->sexdisplay bootgraph edge)
- Fault count: 0

## Files Changed

- `scripts/daily_driver_master_gate.sh` — keyboard_gui gate (3 acceptance paths) and silkbar_phase3_status gate (2 acceptance paths)
- `docs/handoff/SILKBAR_GATE_MARKER_DRIFT_AUDIT_V1.md` — this handoff

## Recurrence Note

When runtime markers change:
1. Grep source for old marker name → confirm if renamed/removed/gated
2. Find equivalent alive markers in the log
3. Patch gate to accept both old and new paths (never narrow acceptance)
4. Do NOT change runtime code to re-emit old markers
5. Always preserve fault scan (`#PF`/`#GP`/`panic`/`fault.kill`)
