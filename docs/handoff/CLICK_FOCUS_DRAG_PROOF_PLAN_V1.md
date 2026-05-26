# CLICK_FOCUS_DRAG_PROOF_PLAN_V1

A) PASS / FAIL / PARTIAL
- PASS (proof plan complete; docs-only phase)
- Scope honored: no runtime behavior change, no kernel/ABI/sex-pdx edits.

B) current click marker/gate map

Current click markers (present)
- `sexinput` normalizer/button evidence:
  - `[sexinput.pointer.button.down] btn=<id> pressed=1`
  - `[sexinput.pointer.button.up] btn=<id> pressed=0`
  - `normalize_pointer_report_v1(...)` masks to `buttons & 0x07`, XOR edge detect, emits only edge transitions (`EV_BTN` ids 1..3).
- `silk-shell` button/click evidence:
  - `[silk-shell.pointer.recv] class=EV_BTN btn=<id> pressed=<0|1>`
  - `[shell.pointer.button] btn=<id> down=<0|1> x=<x> y=<y>`
  - `[silk-shell.click.down] btn=1 x=<x> y=<y> buttons=<mask>`
  - `[silk-shell.click.up] btn=1 x=<x> y=<y>`
  - `[shell.click_focus.down] x=<x> y=<y> buttons=<mask>`
  - `[shell.click_focus.hit] id=<sid>` / `[shell.click_focus.miss]`
  - `[shell.click_focus.send.start] id=<sid>` / `[shell.click_focus.send.ok] id=<sid>`
  - `[shell.click.real.target] x=<x> y=<y> target=<id> kind=<kind>`
  - AP4 contract lane marker: `[shell.interact.stage.click_focus] target=<id> kind=<kind> old_focus=<id> new_focus=<id> ok=<0|1>`

Current click-related gates (present)
- `shell_interaction_contract` (requires `shell.interact.contract.begin`, includes click stage marker)
- `shell_interaction_pointer_no_focus_mutation`
- `shell_interaction_dead_target_guard`
- `faults_zero`

Click-proof gaps for AP8
- No dedicated AP8 click gate rows yet for edge semantics and explicit hit-test->commit chain.
- No dedicated begin marker family for AP8 click lane; existing AP4 begin marker is broader.

C) current drag marker/gate map

Current drag markers (present)
- shell drag lifecycle:
  - `[shell.drag.pending] target=<id> kind=<kind> start_x=<x> start_y=<y> buttons=<mask>`
  - `[shell.drag.threshold] ... pass=<0|1>`
  - `[shell.interact.drag.begin] sid=<id> x=<x> y=<y>`
  - `[shell.interact.drag.move] sid=<id> dx=<dx> dy=<dy>`
  - `[shell.drag.update] sid=<id> ...`
  - `[shell.interact.drag.end] sid=<id> x=<x> y=<y>`
  - `[shell.drag.end] sid=<id> frame=0 x=<x> y=<y>`
- AP4 contract lane drag capture:
  - `[shell.interact.stage.drag_capture] phase=begin target=<id> live=<0|1> capture=1 release=0 ok=1`
  - `[shell.interact.stage.drag_capture] phase=move target=<id> live=<0|1> capture=1 release=0 ok=1`
  - `[shell.interact.stage.drag_capture] phase=release target=<id> live=<0|1> capture=0 release=1 ok=1`
- dead/liveness guards used by drag:
  - `[shell.interact.stage.dead_target_guard] kind=drag target=<id> action=cancel ok=1`
  - legacy guard markers: `shell.tile.skip_dead`, `shell.surface.drag.cancel.dead`, `shell.hover.clear.dead`, `tiling.focus.clear`
- atlas real pointer drag/drop markers (separate lane, already gate-covered):
  - `[silk.atlas.pointer.drag.begin] ...`
  - `[silk.atlas.pointer.drop.done] ...`
  - `[silk.atlas.pointer.drop.reject] ...`
  - `[silk.atlas.pointer.event.consume] kind=<down|up> ok=1`

Current drag-related gates (present)
- `shell_interaction_contract` (requires `shell.interact.stage.drag_capture`)
- `atlas_phase_e3_drag_begin_marker`
- `atlas_phase_e4d_real_pointer_drop`
- `faults_zero`

Drag-proof gaps for AP8
- No dedicated shell click/drag AP8 gate rows for capture lifecycle and release-clears-capture semantics by AP8 naming.

D) missing proof markers

Required AP8 markers vs current state
- `[click.focus.proof.begin]` -> MISSING (proposed new AP8 begin marker)
- `[click.focus.button.down]` -> MISSING (closest existing: `shell.pointer.button` + `silk-shell.click.down`)
- `[click.focus.hit_test]` -> MISSING (closest existing: `shell.click_focus.hit/miss`)
- `[click.focus.commit]` -> MISSING (closest existing: `shell.click_focus.send.ok` + `shell.interact.stage.click_focus`)
- `[click.focus.button.up]` -> MISSING (closest existing: `silk-shell.click.up`)
- `[drag.proof.begin]` -> MISSING (proposed AP8 begin marker)
- `[drag.capture.begin]` -> MISSING (closest existing: `shell.interact.stage.drag_capture phase=begin`)
- `[drag.capture.move]` -> MISSING (closest existing: `shell.interact.stage.drag_capture phase=move`)
- `[drag.capture.release]` -> MISSING (closest existing: `shell.interact.stage.drag_capture phase=release`)
- `[drag.proof.done]` -> MISSING (proposed AP8 done marker)

Reuse policy
- AP8 should reuse existing `shell.interact.stage.*`, `shell.click_focus.*`, and `shell.pointer.button` evidence where it proves required facts.
- Add AP8-named markers only for facts not already explicit (proof boundaries + concise edge/commit summaries).

E) proposed AP8 marker names
- `[click.focus.proof.begin] mode=proof ok=1`
- `[click.focus.button.down] btn=<1|2|3> edge=1 x=<x> y=<y> ok=1`
- `[click.focus.hit_test] x=<x> y=<y> target=<id|0> kind=<kind|none> old_focus=<id> ok=1`
- `[click.focus.commit] target=<id|0> old_focus=<id> new_focus=<id> changed=<0|1> reason=<hit|miss|background> ok=1`
- `[click.focus.button.up] btn=<1|2|3> edge=1 x=<x> y=<y> ok=1`
- `[drag.proof.begin] mode=proof ok=1`
- `[drag.capture.begin] target=<id> live=<0|1> capture=1 ok=1`
- `[drag.capture.move] target=<id> live=<0|1> dx=<dx> dy=<dy> moved=<0|1> ok=1`
- `[drag.capture.release] target=<id|0> live=<0|1> capture=0 release=1 stale=0 ok=1`
- `[drag.proof.done] begin=1 move=1 release=1 faults=0 ok=1`

F) proposed AP8 gate names
- `click_focus_button_edges`
  - PASS: down+up edge markers exist, btn mapping in 1..3, no repeated-state fake edge evidence.
  - SKIP: AP8 begin missing.
  - FAIL: begin present but edge evidence incomplete or invalid button mapping.
- `click_focus_hit_test_commit`
  - PASS: pointer loc + hit-test + old/new focus + commit marker chain.
  - SKIP: AP8 begin missing.
  - FAIL: begin present, missing hit-test/commit evidence.
- `drag_capture_lifecycle`
  - PASS: begin->move->release capture markers with liveness checks.
  - SKIP: drag begin missing.
  - FAIL: begin present but lifecycle incomplete.
- `drag_release_clears_capture`
  - PASS: release marker shows capture=0, stale=0, and no lingering drag target.
  - SKIP: drag begin missing.
  - FAIL: release missing or stale capture evidence.
- `click_drag_dead_target_guard`
  - PASS: dead-target guard markers present (`shell.interact.stage.dead_target_guard` or legacy markers) within AP8 lane.
  - SKIP: AP8 begin missing.
  - FAIL: begin present without dead-target protection evidence.
- `click_drag_faults_zero`
  - PASS: no `#PF/#GP/panic/fault.kill/IPC storm/ring overflow`.
  - SKIP: not used (fault safety should be strict).
  - FAIL: any fault marker.

G) proof lane strategy
- Primary lane: extend existing daily driver gate script with AP8 rows (reuse current `faults_zero` and existing shell/atlas interaction evidence).
- Triggering model: explicit begin markers only; default behavior when begin marker absent must be `SKIP not_requested`.
- Preferred begin sentinels:
  - click lane: `[click.focus.proof.begin]`
  - drag lane: `[drag.proof.begin]`
- Reuse AP4 env where possible (`SEXOS_SHELL_INTERACTION_CONTRACT_PROOF=1`) to avoid new broad proof profile.
- Introduce new AP8 env flags only if AP4 lane cannot isolate AP8 evidence cleanly.
- No full proof run required for this planning phase.

H) STOP FIRST boundaries
- STOP FIRST before touching any of:
  - kernel files (`kernel/src/*`), interrupt/syscall/ABI internals
  - `crates/sex-pdx/*` or sex-pdx opcode/ABI contracts
  - cross-PD IPC protocol changes
  - sexdisplay renderer policy/ownership, framebuffer writer ownership, or bounds-check logic
  - shared-memory/backing-buffer architecture
  - non-shell input policy owners
- AP8 implementation scope should remain inside shell/input proof markers and gate script rows unless blocker proves otherwise.

I) files changed
- `docs/handoff/CLICK_FOCUS_DRAG_PROOF_PLAN_V1.md` (new)

J) next required autopilot
- `CLICK_FOCUS_DRAG_IMPL_V1`

## Notes
- Baseline AP1-AP6 status accepted as provided; unrelated Linen/SexObject failures intentionally ignored for AP8 planning unless they block AP8 lane execution.
- This handoff does not claim click/drag 100%; this is proof-plan only.
