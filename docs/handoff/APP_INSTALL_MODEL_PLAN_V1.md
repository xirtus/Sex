# APP_INSTALL_MODEL_PLAN_V1

Status: docs-only architecture plan (no implementation)

## Goal
Define a real app registry/install/launch model that replaces hardcoded launcher rows while preserving SexOS constraints:
- no POSIX path/process assumptions
- no std/libc/threads assumptions
- no kernel/ABI/sex-pdx edits in this mission

## Baseline Constraints
- Current daily-driver gate baseline: `18/18 PASS`, `faults=0`
- Keyboard-first V1 is proven
- SilkBar ABI Phase 1-5 is proven
- Pointer/USB slot2 remains deferred

## Core Model

### 1) Canonical App Identity (SexObject-owned)
Each installable app is represented by a stable `AppObject` identity in the SexObject model.
Required conceptual fields:
- `app_id` (stable numeric identity)
- `app_kind` (system/user/tool)
- `entry_ref` (capability route token, not POSIX path)
- `display_name`
- `version_tag`
- `state` (`registered|installed|disabled|broken`)

This keeps identity and policy in object space, not filesystem path semantics.

### 2) Linen as Registry View + Operator Surface
Linen provides the queryable/table view over `AppObject` entries.
Linen responsibilities:
- list and filter app records
- expose install status and failure reason fields
- provide keyboard action intents (register/install/enable/disable/launch-request)

Linen is a view/controller surface; it is not the authority for launch policy.

### 3) Launch Contract (PDX-only, capability-gated)
Launch is modeled as a capability-checked request flow, not process/path spawn.
Conceptual flow:
1. caller issues launch intent for `app_id`
2. policy component validates capability grants for requested app
3. shell/runtime adapter resolves `entry_ref`
4. launch request is dispatched through existing PDX contract lanes
5. status/result markers are emitted for shell/spindle/linen visibility

No POSIX exec/spawn assumptions are introduced.

### 4) Install Contract (Object-state transition)
Install is modeled as state transitions on `AppObject` plus manifest metadata linkage.
Conceptual transitions:
- `registered -> installed`
- `installed -> disabled`
- `disabled -> installed`
- `installed -> broken` (on runtime integrity/policy failure)

Install actions should emit audit markers and preserve deterministic state truth in Linen view.

## Proposed Interfaces (Conceptual, no ABI changes here)
- `AppRegistryQuery` (read model)
- `AppInstallIntent` (state transition request)
- `AppLaunchIntent` (runtime launch request)
- `AppAuditEvent` (result/failure evidence)

These are logical contracts only in this plan; implementation must STOP FIRST before ABI/kernel work.

## Phased Rollout (Future Missions)

### Phase A: Read-only Registry Surface
- expose app table in Linen via existing local/object model primitives
- no launch behavior changes
- prove list/filter/status markers

### Phase B: Install State Transitions
- wire install/enable/disable intents to object state updates
- keep launch still hardcoded/fallback during this phase
- prove transition markers and persistence semantics

### Phase C: Launch Intent Bridging
- route launch through capability-checked intent path
- fallback to current hardcoded launcher rows until parity proven
- prove launch success/fail markers per app_id

### Phase D: Hardcoded Row Decommission
- remove/retire hardcoded app mapping only after parity gates pass
- keep emergency fallback switch for one phase

## Risks
- Policy drift between launch intent and capability tables
- Registry view drift if Linen cache and source-of-truth object state diverge
- Launch readiness ambiguity without strict status markers

## Required Proof Markers (Future implementation)
- `[app.registry.row] app_id=N state=NAME ok=N`
- `[app.install.transition] app_id=N old=NAME new=NAME ok=N`
- `[app.launch.intent] app_id=N status=NAME reason=NAME`
- `[app.launch.policy] app_id=N allowed=N reason=NAME`

## STOP FIRST Boundaries
STOP and escalate before implementation if any task requires:
- kernel syscall surface changes
- sex-pdx ABI/type changes
- new shared-memory/backing-buffer protocols
- USB/input/pointer subsystem coupling

## Non-goals in this mission
- No runtime source edits
- No launch path rewiring
- No ABI/version changes
- No test harness changes

## Next Action Suggestion
Implement Phase A as a bounded proof mission under shell/linen only, with daily-driver gate preservation required.
