# USB_SLOT2_MULTI_HID_ARCH_REVIEW_V1

Status: architecture review only (no implementation)

## Goal
Revisit slot2 multi-HID (mouse/keyboard/tablet) architecture after keyboard V1 milestone, with explicit STOP before any kernel/ABI/USB behavior changes.

## Current State (from existing evidence)
- Keyboard-first daily-driver baseline is stable (`18/18 PASS`, `faults=0`).
- Slot2 mouse remains blocked in multi-device lane.
- Prior docs show endpoint/context/doorbell setup for slot2 appears coherent, yet event ring remains dominated by slot1 transfer events.
- Strong indication: blocker is in multi-slot xHCI runtime behavior path, not downstream shell/UI behavior.

Primary references:
- `docs/handoff/USB_SLOT2_MOUSE_BLOCKED_HANDOFF_V1.md`
- `docs/handoff/SEXUSB_HID_MULTIDEVICE_POINTER_AUDIT_V1.md`
- `docs/handoff/SEXUSB_SLOT2_EVENT_DEMUX_STOP_FIRST_V1.md`
- `docs/handoff/SEXUSB_DUAL_DEVICE_QEMU_SLOT2_PROOF_V1.md`

## Invariants To Preserve
- no kernel/ABI/sex-pdx edits in this mission
- no sexusb behavior edits in this mission
- no pointer/path behavior changes in this mission
- sexdisplay remains sole framebuffer writer
- no shared-memory/backing-buffer redesign

## Architecture Hypotheses (Risk-ranked)

### Option A (Lowest risk): Event-demux observability hardening only
Scope:
- tighten architecture-level event ownership map and invariants in docs
- define exact ring/event ownership expectations for slot1 vs slot2

Why:
- provides clearer next diagnostic target without behavior edits

Risk:
- low (documentation and model clarity only)

### Option B (Medium risk): Endpoint lifecycle ordering review spec
Scope:
- produce a sequence-level spec for slot2 endpoint bring-up ordering
- compare expected state transitions against current observed markers

Why:
- likely root-cause area based on coherent descriptors but missing slot2 transfers

Risk:
- medium (can mislead if assumed as implementation without proving)

### Option C (Higher risk): Multi-slot scheduler fairness theory review
Scope:
- review conceptual scheduler/event-ring fairness assumptions when slot1 is active
- identify where starvation or ownership bias could arise

Why:
- aligns with evidence: repeated slot1 events, slot2 silent

Risk:
- high (broad, easy to drift into implementation speculation)

## Recommended Next Mission Shape (for later, not now)
A bounded diagnostics-only mission should:
1. Add/verify slot ownership markers around event read path.
2. Compare slot2 context setup against known-good slot1 at the same lifecycle point.
3. Stop after first missing invariant marker.
4. Avoid retries/"wait harder" loops without new invariants.

## STOP FIRST Gate
Before any follow-up implementation mission proceeds, explicitly approve scope expansion if it touches:
- sexusb runtime behavior
- kernel scheduling or interrupt dispatch
- xHCI command/transfer ring write semantics
- any ABI/pdx surface

If such edits become necessary, stop and create a dedicated STOP-FIRST implementation handoff before code changes.

## Decision
Mission complete as architecture review only.
No code changes proposed or performed.
