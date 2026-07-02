# Sex Microkernel Input Post-Mortem: Why Mouse + Keyboard Still Are Not 100%

## Executive Summary

After three months, Sex Microkernel did not fail because the idea is bad. It failed because the team repeatedly treated partial technical progress as product completion.

The kernel got more stable. Multiple protection domains spawn. The scheduler improved. Display and Silk advanced. But mouse and keyboard still are not “100%” because the project never forced input through a complete, boring, user-visible acceptance path:

real hardware or QEMU device → bus/controller layer → HID decode → normalized event → PDX delivery → shell policy → visible result → sustained runtime proof → negative tests → regression gate.

Instead, the work kept sliding between layers: kernel faults, display rendering, SilkBar clock, shell state, synthetic pointer smoothing, USB planning, and product design. Every one of those mattered, but none of them by itself proved that the user could reliably move, click, type, focus, drag, and recover from bad input.

The root failure was not only technical. It was execution discipline.

## What We Claimed Versus What Was Actually True

The project often used language like “done,” “100%,” or “current-tier complete” when the proof was narrower:

* “Synthetic pointer works” is not “mouse is done.”
* “Keyboard IRQ path exists” is not “keyboard is product-stable.”
* “All PDs spawn” is not “input path is stable.”
* “No normal boot faults” is not “interactive desktop acceptance passed.”
* “Click focus/drag works in a proof lane” is not “real USB mouse works.”
* “QEMU window appears” is not “the GUI is usable.”
* “A plan exists for USB” is not “USB input shipped.”

The most damaging pattern was optimism inflation: we kept upgrading the language faster than the proofs.

## Exact Root Failure

The input stack was never reduced to one canonical product contract.

A correct SexOS input contract should have been:

1. Keyboard produces deterministic keydown/keyup events.
2. Pointer produces deterministic dx/dy or absolute position events.
3. Button down/up is edge-correct and debounced.
4. Events enter `sexinput`.
5. `sexinput` emits normalized HID events only.
6. `silk-shell` owns focus, click, drag, resize, and keyboard shortcuts.
7. `sexdisplay` only renders final pixels and forwards display-facing messages.
8. No kernel, ABI, compositor, or shared-buffer redesign is allowed for normal input fixes.
9. Product acceptance requires visible behavior plus fault scan.
10. “100%” means real or QEMU hardware path, not synthetic-only.

That contract existed in fragments, but it was not enforced as the central release gate.

## Technical Reasons It Failed

### 1. USB was treated like “just input,” but USB is a whole stack.

USB input is not one patch. It requires host controller discovery, XHCI bring-up, device enumeration, endpoint setup, interrupt transfers, HID report parsing, normalization, IPC delivery, and shell behavior. We knew this, but the emotional expectation stayed “mouse should be easy.”

That mismatch created frustration. The correct interpretation is: PS/2 keyboard, QEMU tablet, USB mouse, and trackpad are different tiers.

### 2. Synthetic input masked missing hardware proof.

Synthetic pointer reports were useful, but they became a trap. They proved the downstream path could work, but they did not prove:

* XHCI works.
* USB HID boot protocol works.
* button bits are decoded correctly.
* real interrupt transfers work.
* real report cadence does not flood IPC.
* real devices survive sustained runtime.

Synthetic proof should have been labeled “downstream shell proof only,” never “mouse proof.”

### 3. Click/button proof lagged behind movement proof.

Pointer movement is only half the product. A desktop is unusable without button edge correctness:

* down event
* up event
* no repeated false click
* click focuses correct surface
* click does not leak to wrong surface
* drag starts only after correct down target
* drag ends cleanly on release
* dead surface clears focus/drag state

The project spent too much time celebrating cursor movement and not enough time proving button semantics.

### 4. Keyboard and mouse were mixed conceptually.

Keyboard, PS/2 mouse, USB mouse, USB tablet, and touchpad are not one implementation. They should share the normalization/output contract, but their producers differ.

The failure was trying to think “input is one feature” instead of “input is a pipeline with multiple producers and one normalized event contract.”

### 5. Ownership boundaries were not turned into executable gates early enough.

The intended ownership is right:

* `sexinput` produces normalized input.
* `silk-shell` owns input policy.
* `sexdisplay` owns pixels.
* kernel routes interrupts and protects domains.

But a written ownership rule is not enough. The build gate should have rejected patches that moved input policy into `sexdisplay`, changed `sex-pdx`, touched kernel ABI casually, or mixed USB with gesture policy.

### 6. Too many unrelated failures shared the same emotional bucket.

Clock bugs, SilkBar delivery bugs, null-jump containment, QEMU display launch, window lifecycle, and input all got mentally grouped as “desktop still broken.”

That made prioritization worse. The team kept fixing surrounding symptoms while the actual product test remained simple:

Can I boot, move mouse, click a window, drag it, type text, repeat for 60 seconds, and see zero faults?

That should have been the daily driver gate.

### 7. Runtime proof was weaker than build proof.

Build gates improved. But interactive runtime proof stayed too human, too occasional, and too log-fragile.

A real input completion gate needs markers like:

```text
[input.boot.ready]
[input.keyboard.keydown.ok]
[input.keyboard.keyup.ok]
[input.pointer.move.ok]
[input.pointer.button.down.ok]
[input.pointer.button.up.ok]
[silk.focus.click.ok]
[silk.drag.begin.ok]
[silk.drag.move.ok]
[silk.drag.end.ok]
[input.sustain.ok ticks=... events=...]
[input.faultscan.ok pf=0 gp=0 panic=0 storm=0]
```

Without these, “works on my screen” kept becoming the acceptance test.

### 8. No hard distinction between current-tier 100% and future-tier 100%.

This caused constant scope creep.

Current-tier input 100% should mean:

* QEMU keyboard works.
* QEMU USB tablet or mouse works.
* button click works.
* drag works.
* no faults.
* no IPC storm.
* proof markers exist.
* regression gate exists.

Future-tier 100% can include:

* arbitrary USB device support.
* physical laptop trackpad.
* multitouch.
* gestures.
* Bluetooth.
* hotplug.
* power management.
* per-device settings.
* accessibility remapping.

The project kept emotionally judging current-tier work against future-tier expectations.

## Product Reasons It Failed

### 1. The user-facing promise was too broad.

“Mouse keyboard 100%” sounds simple to a user. But in an OS project it means multiple device classes, bus layers, event semantics, focus policy, compositor interaction, and long-run stability.

The product promise should have been narrower:

“QEMU keyboard + QEMU USB tablet: click, drag, type, no faults.”

That would have produced a clear win.

### 2. The definition of “daily driver” was not ruthless enough.

A desktop is not daily-driver if input is flaky. It does not matter how advanced MPK, PDX, Silk, Linen, or SexNet are. If the mouse and keyboard are not reliable, the product feels broken.

Input should have been promoted above almost every other feature until stable.

### 3. Too much work chased impressive architecture instead of boring usability.

SexOS has advanced ideas: SASOS, MPK isolation, PDX, capability routing, Silk DE. But the first product trust moment is primitive:

The cursor moves. The click lands. The key types. The window responds.

Failing there makes the whole system feel less real, no matter how sophisticated the internals are.

## Marketing Reasons It Failed

### 1. We overused “100%.”

The word “100%” became motivational instead of contractual. That damages trust, internally and externally.

Going forward:

* “PASS” must mean exact gate passed.
* “100% current-tier” must mean all current-tier gates pass.
* “Future-tier deferred” must be stated clearly.
* “Synthetic proof” must never be marketed as hardware proof.

### 2. We sold the vision before locking the primitive.

A big vision is good. But public messaging should separate:

* vision
* current working proof
* next gate
* known limitation

Better wording:

“Sex Microkernel now boots six isolated PDs and has a synthetic input-to-shell path. The next milestone is boring but crucial: real QEMU USB pointer and keyboard acceptance with click, drag, type, and zero faults.”

That is credible.

### 3. The narrative jumped between OS superiority and basic usability.

When a project says it is superior to Linux but the mouse is not finished, users lose trust. The marketing has to become more humble and more proof-driven.

The best story now is not “we already won.” It is:

“We are building a radically different OS, and we are now forcing every subsystem through brutal proof gates before calling it done.”

That is stronger.

## User Behavior Reasons It Failed

### 1. Users judge input instantly.

Users do not care that the scheduler is elegant if the cursor jumps, freezes, misses clicks, or fails to type. Input is perceived as the nervous system of the OS.

A single missed click feels worse than ten invisible kernel improvements.

### 2. Users expect commodity reliability.

Mouse and keyboard are solved problems in mainstream OSes. Users will not give much patience here. They may forgive missing apps. They will not forgive unreliable input.

### 3. Developers also need input to build confidence.

Even for us, being unable to interact smoothly with QEMU makes every visual test emotionally expensive. Bad input slows debugging, demos, recordings, and motivation.

## Competition Reasons It Failed

### 1. Competitors are boring and reliable.

Linux, BSDs, Windows, macOS, Redox-style projects, and hobby kernels all compete indirectly at the input layer. Their advantage is not conceptual purity. It is that keyboards and mice usually work.

SexOS can be more interesting architecturally, but it cannot out-explain broken input.

### 2. Rival systems have mature fallback paths.

Mainstream systems have layers of fallback: BIOS/UEFI input, PS/2, evdev/libinput, HID quirks, device databases, compositors, accessibility remappers.

SexOS intentionally avoids Linux assumptions, but that means we must replace convenience with explicit proof.

### 3. A rival would beat us by narrowing scope.

The rival would not try to finish the whole desktop. They would ship one airtight demo:

* boot
* move
* click
* type
* drag
* open menu
* no crash
* repeat every build

That would look more real than a grander but unstable system.

## Timing Reasons It Failed

### 1. Input should have been frozen before Silk polish.

Silk visual polish, Linen work, Clock/SilkBar, SexNet, and file work all matter. But input should have become the top blocker earlier.

Correct priority:

1. boot stable
2. display visible
3. input stable
4. shell focus/drag stable
5. only then visual polish

We kept allowing parallel ambition after input was already late.

### 2. Three months is long enough to expose process failure.

A hard bug can take time. But three months without a finished mouse/keyboard path means the issue is not just hard code. It is failure to isolate the finish line.

## Execution Reasons It Failed

### 1. The agents were given too many broad missions.

Prompts sometimes mixed:

* USB
* HID
* gestures
* compositor
* display
* kernel
* shell policy
* build scripts
* handoff docs

That is exactly how worker models drift.

### 2. STOP FIRST rules were present but not always used as product gates.

“No kernel/ABI edits unless STOP FIRST” is good. But the release process needed more:

* no input patch touching more than two ownership layers without STOP FIRST
* no “done” without visible proof
* no “100%” without negative tests
* no USB producer work before host discovery proof
* no gesture work before click proof

### 3. Handoffs were too optimistic.

Handoffs often captured useful detail, but some framed progress as completion. The handoff should always answer:

* what is proven
* what is not proven
* what is synthetic
* what is hardware
* what can regress
* exact next gate

### 4. The team kept debugging symptoms instead of installing acceptance gates.

Every recurring failure should have become a gate. Instead, many became another handoff and another prompt.

A bug fixed once is progress. A bug fixed and converted into a regression gate is engineering.

## The Brutal Conclusion

The mouse/keyboard stack is not 100% because we did not make it impossible to call it 100% while it was still synthetic, partial, or visually unproven.

The failure was:

* not enough product ruthlessness
* not enough proof discipline
* too much scope mixing
* too much “almost done” language
* too much architecture pride
* not enough boring input acceptance

The fix is not to work harder randomly. The fix is to narrow the milestone, freeze the boundaries, and make every input claim executable.

# Revised Strategy: What We Change Right Now

## New Rule: Input Becomes Release Blocker #1

Until current-tier input passes, no new Silk polish, Linen feature, Bell feature, Mesh feature, visual theme work, or broad OS roadmap work should be considered higher priority.

Allowed parallel work only:

* docs cleanup
* proof gate scripts
* tiny non-invasive diagnostics
* review prompts

## Define Current-Tier Input 100%

Current-tier 100% means exactly this:

```text
INPUT_100_CURRENT_TIER_V1

Device scope:
- QEMU keyboard
- QEMU USB tablet or USB mouse
- optional PS/2 keyboard if already present

Behavior scope:
- keydown recognized
- keyup recognized
- pointer movement recognized
- button down recognized
- button up recognized
- click focuses a live surface
- drag begins on button down over draggable surface
- drag moves the surface
- drag ends on button up
- repeated clicks do not crash
- repeated moves do not storm IPC
- dead/invalid target does not poison focus
- system runs 60 seconds after interaction

Proof scope:
- build PASS
- boot PASS
- runtime marker PASS
- negative marker PASS
- fault scan PASS
- handoff updated
```

Anything beyond that is future-tier.

## Freeze Ownership

Input ownership is now locked:

```text
kernel:
  interrupt routing, capability checks, scheduling, fault containment only

sexusb:
  future USB bus owner only

sexinput:
  hardware/input producer
  HID decode
  normalization
  OP_HID_EVENT emission

silk-shell:
  focus policy
  click policy
  drag policy
  keyboard shortcuts
  target liveness checks

sexdisplay:
  sole framebuffer writer
  render only
  no input policy
```

Forbidden in current-tier input work:

```text
- kernel ABI edits unless STOP FIRST
- sex-pdx ABI edits unless STOP FIRST
- display ownership changes
- shared framebuffer redesign
- gesture implementation
- trackpad multitouch
- Bluetooth
- app launcher expansion
- shell visual redesign
- broad refactor
```

## New Immediate Phase Sequence

### Phase 0 — Baseline Truth Audit

Goal: stop lying to ourselves.

Actions:

* capture current branch, commit, dirty tree
* run build
* boot QEMU
* record whether keyboard works
* record whether pointer moves
* record whether click works
* record whether drag works
* record exact serial fault scan
* write `docs/handoff/INPUT_BASELINE_TRUTH_V1.md`

Output must say PASS, FAIL, or SKIP for every item.

### Phase 1 — Input Contract Lock

Goal: one normalized event contract.

Actions:

* document exact `OP_HID_EVENT` fields
* document key event values
* document pointer dx/dy values
* document button bit values
* document wheel value if present, but do not implement new wheel behavior yet
* add static constants if missing
* no ABI change unless STOP FIRST

Output:

* `docs/handoff/INPUT_EVENT_CONTRACT_V1.md`
* grep proof that producer and consumer use the same constants

### Phase 2 — Keyboard Acceptance

Goal: keyboard is boring.

Actions:

* prove keydown
* prove keyup
* prove repeat does not flood
* prove shortcut path if currently supported
* no pointer work in this phase

Markers:

```text
[input.keyboard.begin]
[input.keyboard.keydown.ok]
[input.keyboard.keyup.ok]
[input.keyboard.repeat.bounded.ok]
[input.keyboard.done]
```

### Phase 3 — Pointer Movement Acceptance

Goal: movement only.

Actions:

* prove normalized dx/dy
* prove cursor/shell state updates
* prove bounds clamp
* prove no IPC storm
* no button/click changes in this phase

Markers:

```text
[input.pointer.move.begin]
[input.pointer.report.recv]
[input.pointer.normalized.ok]
[silk.pointer.state.ok]
[input.pointer.bounds.ok]
[input.pointer.move.done]
```

### Phase 4 — Button Edge Acceptance

Goal: button down/up only.

Actions:

* decode button bit
* emit button down once
* emit button up once
* reject duplicate stuck edge
* no drag policy changes yet

Markers:

```text
[input.button.begin]
[input.button.down.ok]
[input.button.up.ok]
[input.button.edge.once.ok]
[input.button.done]
```

### Phase 5 — Click Focus Acceptance

Goal: click focuses live surface.

Actions:

* button down over target
* shell hit-test selects target
* focus changes only if target is live
* invalid/dead target rejected safely
* no drag yet

Markers:

```text
[silk.click.begin]
[silk.click.hit.live.ok]
[silk.focus.set.ok]
[silk.click.dead_target.reject.ok]
[silk.click.done]
```

### Phase 6 — Drag Acceptance

Goal: drag lifecycle is complete.

Actions:

* down over draggable surface starts drag
* movement updates surface
* release ends drag
* focus/drag clears if target dies
* no gestures

Markers:

```text
[silk.drag.begin.ok]
[silk.drag.move.ok]
[silk.drag.end.ok]
[silk.drag.dead_target.clear.ok]
[silk.drag.done]
```

### Phase 7 — Integrated 60-Second Runtime Gate

Goal: product confidence.

Scenario:

1. boot
2. wait for all PDs
3. move pointer
4. click focus
5. drag window
6. type keys
7. repeat interaction burst
8. run 60 seconds
9. scan logs

Markers:

```text
[input.integration.begin]
[input.integration.keyboard.ok]
[input.integration.pointer.ok]
[input.integration.click.ok]
[input.integration.drag.ok]
[input.integration.sustain.ok]
[input.integration.faultscan.ok]
[input.integration.done]
```

Required final line:

```text
INPUT_100_CURRENT_TIER_V1: PASS
```

## Stop Using “100%” Until the Final Marker Exists

Allowed language:

* “keyboard producer PASS”
* “synthetic pointer PASS”
* “QEMU pointer movement PASS”
* “button edge FAIL”
* “click focus SKIP”
* “current-tier input not complete”

Forbidden language:

* “mouse is basically done”
* “keyboard is 100%”
* “input complete except USB”
* “desktop usable” without integrated proof

## Revised Product Messaging

Use this publicly:

```text
Sex Microkernel is tightening its proof discipline.

The kernel now boots the core isolated PDs, but we are no longer calling desktop features complete until they pass user-visible runtime gates.

The current blocker is input. The next milestone is intentionally boring:
QEMU keyboard + pointer must move, click, focus, drag, type, and sustain runtime with zero faults.

No more “100%” without proof.
```

## Revised Agent Workflow

Use fewer, stricter prompts.

### Claude: audit and invariants.

Use Claude for contracts, proof plans, and review.

### Codex: minimal implementation.

Use Codex for small patches only.

### Gemini/local agents: investigation.

Use local agents for grep, logs, QEMU traces, and narrow reports.

### No agent gets a broad “fix input” prompt.

Every prompt must name the phase and forbidden edits.

# Immediate Bash-Friendly Prompts

## Prompt 1: Baseline Truth Audit

```bash
cat > /tmp/claude_input_baseline_truth_v1.prompt <<'EOF'
MISSION: INPUT_BASELINE_TRUTH_V1.

BACKUP BEFORE CHANGES.
If something goes wrong: READ HANDOUTS and .mds first.
Reduce token waste: rg first, inspect small snippets only, no broad dumps.
Save recurring fixes/issues in docs/handoff.

NO Linux assumptions. NO POSIX.
Strict no_std Rust Sex Microkernel.
PDX only. MPK/PKU/PKEY isolation.
sexdisplay sole framebuffer writer.
silk-shell owns shell/session/input policy.
No kernel/ABI/sex-pdx edits unless STOP FIRST.
Preserve framebuffer bounds checks.
No shared-memory/backing-buffer redesign.
No broad refactor.

TASK:
Audit the current input reality. Do not patch feature code.

Check:
1. current branch/commit/dirty tree
2. build result
3. QEMU boot result
4. all PD spawn result
5. keyboard keydown/keyup proof if present
6. pointer movement proof if present
7. button down/up proof if present
8. click focus proof if present
9. drag proof if present
10. fault scan: #PF/#GP/panic/fault.kill/IPC storm

Output:
- PASS/FAIL/SKIP table
- exact missing proof markers
- exact next smallest phase
- write docs/handoff/INPUT_BASELINE_TRUTH_V1.md

Do not implement fixes.
EOF
claude --bare < /tmp/claude_input_baseline_truth_v1.prompt
```

## Prompt 2: Input Contract Lock

```bash
cat > /tmp/claude_input_event_contract_v1.prompt <<'EOF'
MISSION: INPUT_EVENT_CONTRACT_V1.

BACKUP BEFORE CHANGES.
If something goes wrong: READ HANDOUTS and .mds first.
Reduce token waste: rg first, inspect small snippets only, no broad dumps.
Save recurring fixes/issues in docs/handoff.

NO Linux assumptions. NO POSIX.
Strict no_std Rust Sex Microkernel.
PDX only. MPK/PKU/PKEY isolation.
sexdisplay sole framebuffer writer.
silk-shell owns shell/session/input policy.
No kernel/ABI/sex-pdx edits unless STOP FIRST.
Preserve framebuffer bounds checks.
No shared-memory/backing-buffer redesign.
No broad refactor.

TASK:
Lock the current normalized input contract without redesign.

Inspect only:
- servers/sexinput/src/main.rs
- servers/silk-shell/src/main.rs
- servers/sexdisplay/src/main.rs only if HID forwarding is relevant
- crates/sex-pdx/src/lib.rs only for constants/ABI inspection, no edit unless STOP FIRST
- docs/handoff input docs

Find:
1. OP_HID_EVENT identity
2. keyboard event format
3. pointer movement format
4. button bit/down/up format
5. current producer/consumer mismatch
6. missing constants or duplicated magic values

Output:
- docs/handoff/INPUT_EVENT_CONTRACT_V1.md
- mismatch table
- smallest safe patch plan
- STOP FIRST if ABI change is required

Do not implement broad refactor.
EOF
claude --bare < /tmp/claude_input_event_contract_v1.prompt
```

## Prompt 3: Button + Click Proof Implementation

```bash
cat > /tmp/codex_input_button_click_current_tier_v1.prompt <<'EOF'
MISSION: INPUT_BUTTON_CLICK_CURRENT_TIER_V1.

BACKUP BEFORE CHANGES.
If something goes wrong: READ HANDOUTS and .mds first.
Reduce token waste: rg first, inspect small snippets only, no broad dumps.
Save recurring fixes/issues in docs/handoff.

NO Linux assumptions. NO POSIX.
Strict no_std Rust Sex Microkernel.
PDX only. MPK/PKU/PKEY isolation.
sexdisplay sole framebuffer writer.
silk-shell owns shell/session/input policy.
No kernel/ABI/sex-pdx edits unless STOP FIRST.
Preserve framebuffer bounds checks.
No shared-memory/backing-buffer redesign.
No broad refactor.

SCOPE:
Implement only current-tier button edge + click focus proof.
Allowed files:
- servers/sexinput/src/main.rs
- servers/silk-shell/src/main.rs
- scripts/daily_driver_master_gate.sh or current gate script
- docs/handoff/INPUT_BUTTON_CLICK_CURRENT_TIER_V1.md

Do not touch:
- kernel
- sex-pdx ABI
- sexdisplay rendering policy
- compositor framebuffer ownership
- gestures
- trackpad multitouch
- USB XHCI broad implementation
- Linen/SilkBar/SexFiles

TASK:
1. Prove button down event.
2. Prove button up event.
3. Prove duplicate edge is bounded/rejected.
4. Prove click focuses live shell target.
5. Prove dead/invalid target is rejected safely.
6. Add runtime markers:
   [input.button.begin]
   [input.button.down.ok]
   [input.button.up.ok]
   [input.button.edge.once.ok]
   [silk.click.begin]
   [silk.click.hit.live.ok]
   [silk.focus.set.ok]
   [silk.click.dead_target.reject.ok]
   [input.button_click.done]
7. Add/extend gate to fail if markers missing.
8. Run build + runtime gate if available.
9. Fault scan for #PF/#GP/panic/fault.kill/IPC storm.

Return:
A) PASS/FAIL
B) files changed
C) exact root cause if failed
D) markers observed
E) fault scan
F) handoff path
EOF
codex exec < /tmp/codex_input_button_click_current_tier_v1.prompt
```

## Prompt 4: Integrated Input 100 Gate

```bash
cat > /tmp/codex_input_100_current_tier_gate_v1.prompt <<'EOF'
MISSION: INPUT_100_CURRENT_TIER_GATE_V1.

BACKUP BEFORE CHANGES.
If something goes wrong: READ HANDOUTS and .mds first.
Reduce token waste: rg first, inspect small snippets only, no broad dumps.
Save recurring fixes/issues in docs/handoff.

NO Linux assumptions. NO POSIX.
Strict no_std Rust Sex Microkernel.
PDX only. MPK/PKU/PKEY isolation.
sexdisplay sole framebuffer writer.
silk-shell owns shell/session/input policy.
No kernel/ABI/sex-pdx edits unless STOP FIRST.
Preserve framebuffer bounds checks.
No shared-memory/backing-buffer redesign.
No broad refactor.

TASK:
Create the final current-tier input acceptance gate.

Gate requires:
- build PASS
- boot PASS
- all required PDs spawn
- keyboard keydown/up markers
- pointer movement markers
- button down/up markers
- click focus markers
- drag begin/move/end markers
- 60s sustain marker or nearest existing deterministic runtime equivalent
- no #PF
- no #GP
- no panic
- no fault.kill
- no IPC storm

Final success marker:
[input.100.current_tier.pass]

Allowed edits:
- scripts/gate files
- docs/handoff/INPUT_100_CURRENT_TIER_GATE_V1.md
- minimal marker additions only if existing code already has the behavior

STOP FIRST if behavior is missing and requires feature implementation.

Return:
A) PASS/FAIL/SKIP
B) missing markers
C) exact command to run
D) exact final acceptance definition
E) handoff path
EOF
codex exec < /tmp/codex_input_100_current_tier_gate_v1.prompt
```

# Final Operational Change

From now on, the project should not ask:

“Are we 100%?”

It should ask:

“Which exact gate failed?”

For input, the only acceptable final answer is:

```text
INPUT_100_CURRENT_TIER_V1: PASS
build=PASS boot=PASS keyboard=PASS pointer=PASS button=PASS click=PASS drag=PASS sustain=PASS faults=0
```

Until that line exists, mouse and keyboard are not 100%.
