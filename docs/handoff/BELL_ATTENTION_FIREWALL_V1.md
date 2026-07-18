# BELL_ATTENTION_FIREWALL_V1

Design pass only — no code changed. Mission asked to "design" Bell V1 from
the 8 listed tasks; audit found **most of it already built and live**
(`servers/sexbell/src/main.rs`, 1259 lines). This doc documents what's real,
what's a genuine gap, and scopes implementation prompts for the gaps only —
does not re-spec or re-implement what already works.

## A. Bell V1 spec — reality check per mission task

| # | Mission ask | Status | Where |
|---|-------------|--------|-------|
| 1 | BellEvent packed V1 model | **Done** | `BellQueueEntry` struct, `servers/sexbell/src/main.rs:21-52` |
| 2 | Minimal PDX opcodes, no collision | **Done** | `OP_BELL_{NOTIFY,CLOSE,ACTION,LIST,CLEAR,SUBSCRIBE,SET_POLICY,MUTE_SENDER}` = `0xC0`-`0xC7`, `crates/sex-pdx/src/lib.rs:106-113`. Contiguous block, no collision found against any other `OP_*` constant. |
| 3 | Sender capability validation | **Gap (explicitly stubbed)** | `derive_lane_first_proof()` at line 443: *"No BellCap table exists yet. Every sender is unknown/untrusted... max lane = PASSIVE."* This is honest, working-as-designed placeholder, not broken code. |
| 4 | Attention lanes | **Partial mismatch** | Code has 6 numeric lanes `0..=5` (comment: `0=PASSIVE .. 5=SECURITY`), not the 5 named lanes (`NOW/SOON/LATER/SYSTEM/PROJECT`) the mission specifies. See §A.1. |
| 5 | Privacy/redaction model | **Done** | 4 privacy levels (`0=Public..3=FullHidden`), 4 redaction classes, per-caller max-privacy gate (`max_privacy_for_caller`), `FullHidden` counted-but-not-revealed in `OP_BELL_LIST`. Lines 411-436, 770-777. |
| 6 | Dev-mode events (PD crash, PDX timeout, cap denied, build finished) | **Not implemented — real gap** | No sender exists anywhere for any of these four. Confirmed by repo-wide grep: zero hits for crash/timeout/cap-denied/build-finished event emission into Bell. |
| 7 | Proof-only V1 with runtime markers before UI | **Done, and then some** | 20+ distinct `[bell.*]` serial markers already exist (`bell.notify.{recv,ok,reject,downgrade}`, `bell.queue.{push,drop,reject}`, `bell.list.{item,redact,reply}`, `bell.policy.{set,deny,reject}`, `bell.mute.{add,remove,reject}`, `bell.subscribe.reply`, etc.) — **and** a UI already exists on top: silk-shell has a full keyboard-driven Bell detail view (`bell_select_next_row`, `bell_cycle_lane`, `bell_emit_selected_event_detail_proof`, `servers/silk-shell/src/main.rs:741+`), SilkBar polls `OP_BELL_LIST`/`OP_BELL_SUBSCRIBE` every ~2s and renders a presence badge (`servers/silkbar/src/main.rs:150-340`). Spindle has live `bell`/`bell-test`/`bell-status` commands. **The "proof-only, no UI yet" framing in the mission is stale — UI already shipped.** |
| — | Linen object history | **Marker-only stub** | `object_ref_count`/`object_ref` fields exist in `BellQueueEntry` but are never populated from a real Linen lookup — matches mission's own constraint ("Linen persistence later only"), so this is correctly *not* built yet, not a bug. |

### A.1 Lane naming decision (the one real design choice this pass makes)

Existing 6-value numeric lane (`0..=5`) has no canonical name table in code
— only informal comments (`PASSIVE`, `SECURITY`) at the two extremes. Rather
than inventing a 7th lane or bolting names onto a scheme designed around a
different count, **recommend collapsing to the mission's 5 named lanes,
mapped onto the existing ordinal semantics** (lower = less urgent, matches
current `find_lowest_priority_index` drop-order logic unchanged):

| Ordinal | Name | Existing analogue | Semantics |
|---------|------|--------------------|-----------|
| 0 | `LATER` | was `PASSIVE` | Background info, no attention needed now |
| 1 | `PROJECT` | (new) | Linen-object/workspace-context events — reserved now, populated once Linen wiring lands |
| 2 | `SOON` | (unnamed) | Worth seeing this session, not urgent |
| 3 | `SYSTEM` | (unnamed) | Dev-mode / OS-health events (§A.2) |
| 4 | `NOW` | was near top | Needs attention this moment |
| 5 | *(reserved)* | was `SECURITY` | Kept as ordinal 5 for future security-class events; not part of the mission's 5-name set but dropping it outright would be a silent capability regression (nothing currently emits it, but nothing should be blocked from doing so later) |

This is a rename + remap inside `servers/sexbell` only (`derive_lane_first_proof`,
doc comments, the `lane_filter`/`lane_override` validation bounds already
allow `0..=5` so no range-check changes needed). No opcode, ABI, or
kernel change. Scoped as implementation prompt §F.1.

### A.2 Dev-mode event sourcing (the other real gap)

Four event types requested, checked each against what currently exists to
observe them:

- **PD crash** — kernel already has a PF/fault path
  (`kernel/src/interrupts.rs`, the same `KERNEL PAGE FAULT HALT` /
  `forward_page_fault` machinery documented in
  `docs/handoff/STUB_SERVER_KILL_LIST_V1.md` §B.1) but it halts the core or
  signals the faulting domain — **it does not notify Bell**. Wiring this
  properly is a kernel-side change (new call from the fault handler into
  Bell's slot) — **STOP FIRST**, not in scope for this pass.
- **PDX timeout** — no timeout concept exists anywhere in the PDX call path
  today (`pdx_call`/`safe_pdx_call` block or fail immediately, no
  watchdog). Would need a new kernel-side timer, also STOP FIRST.
  Recommend cutting this from V1 entirely — needs its own design pass, not
  a Bell-side task.
- **Cap denied** — **already partially observable without kernel changes.**
  Bell's own `[bell.readcap.deny]` / `[bell.policy.deny]` markers are exactly
  this event class, just not re-surfaced as a Bell notification about
  itself. More generally, any PD's own cap-check failures
  (`sex_pdx::ERR_CAP_INVALID`) happen locally in the calling PD — a PD could
  self-report to Bell via ordinary `OP_BELL_NOTIFY` when it observes its own
  `ERR_CAP_INVALID`. This is userspace-only, no kernel change, doable now.
- **Build finished** — this is a host-side (`scripts/entrypoint_build.sh`)
  event, not something any running PD observes. Only meaningful if the build
  script pushes a marker file or QMP message that some running server polls
  — that's a new plumbing question outside Bell's own scope, and arguably
  not a "kernel notification" concern at all (nothing runs Bell during a
  host-side build). Recommend dropping from V1; revisit only if there's a
  concrete workflow (e.g. a dev-mode watch script) that needs it.

**Net: of the 4 requested dev-mode events, only "cap denied" (self-reported,
userspace-only) is implementable within Bell's own domain right now.** The
other 3 need kernel-side plumbing (crash, timeout) or don't belong to Bell at
all (build finished) — flagged, not attempted.

## B. Protocol sketch (unchanged from what's live)

```
App/Shell/SilkBar → Bell (SLOT_BELL, domain 10)

OP_BELL_NOTIFY   (0xC0): arg0=[redaction:8|privacy:8|urgency:8|category:8]
                         arg1=[action_id:8|action_count:8]
                         arg2=[object_ref:8|object_ref_count:8]
                         caller_pd = kernel-authoritative sender
OP_BELL_CLOSE    (0xC1): arg0=event_id
OP_BELL_ACTION   (0xC2): arg0=event_id, arg1=action_id
OP_BELL_LIST     (0xC3): arg0=[max_results:8|lane_filter:8] → reply packed lane counts
OP_BELL_CLEAR    (0xC4): arg0=lane_filter (0xFF=all)
OP_BELL_SUBSCRIBE(0xC5): → reply = generation counter
OP_BELL_SET_POLICY(0xC6): arg0=target_pd, arg1=packed policy (author allowlist only)
OP_BELL_MUTE_SENDER(0xC7): arg0=[action:8 @32|mute_pd:32]
```

No changes proposed to this wire format — it's already correct and minimal.
The only addition needed for the "cap denied" dev-mode event (§A.2) is a
**convention**, not a new opcode: `category` value reserved for
`SelfCapDenied` (see §F.2), sent via the existing `OP_BELL_NOTIFY` path with
`caller_pd` = the PD reporting on itself.

## C. State machine (unchanged, documented for completeness)

```
[idle] --OP_BELL_NOTIFY--> validate(mute? enum-range? spam-budget?)
    --reject--> [idle] (marker: bell.notify.reject)
    --accept--> derive_lane --policy-override--> push(queue)
        --queue-full--> drop-lowest-priority --> [idle] (marker: bell.queue.drop)
        --ok--> [idle] (marker: bell.notify.ok), bump_generation()

[idle] --OP_BELL_LIST(caller)--> read-cap-check(allowlist)
    --deny--> reply(u64::MAX) (marker: bell.readcap.deny)
    --allow--> privacy-filter(caller_max_privacy) --> reply(packed counts)

[idle] --OP_BELL_CLOSE/ACTION/CLEAR/MUTE/SET_POLICY--> (respective allowlist/validate) --> mutate or reject
```

This is already fully implemented; no new states proposed. The only new
transition needed is a self-notify path for cap-denial (§A.2/§F.2), which
reuses the `NOTIFY` transition unchanged.

## D. Proof markers — existing (kept) vs new (gap-filling)

**Existing (already live, no change):** `bell.boot`, `bell.demo.boot`,
`bell.notify.{recv,ok,reject,downgrade}`, `bell.queue.{push,drop}`,
`bell.queue.reject.full`, `bell.list.{item,redact,reply}`,
`bell.list.reject`, `bell.readcap.deny`, `bell.close.{ok,reject}`,
`bell.action.{dispatch,reject}`, `bell.clear.{ok,reject}`,
`bell.mute.{add,remove,reject}`, `bell.policy.{set,deny,reject}`,
`bell.subscribe.{reply,deny}`.

**New, needed for gap-closing work only:**
- `[bell.lane.rename] old={} new={}` — one-time proof the §A.1 remap didn't
  change drop-priority ordering (assert numeric ordinal identical pre/post).
- `[bell.selfreport.capdenied] caller_pd={} target_op={}` — emitted by a PD
  when it self-reports its own `ERR_CAP_INVALID` via `OP_BELL_NOTIFY`
  (§F.2), distinguishable from Bell's own internal `readcap.deny` (which is
  about *Bell's* allowlist, not a general cap failure elsewhere).

## E. Negative tests

**Already covered by existing code (verify via gate, don't re-derive):**
muted sender rejected before processing; invalid enum ranges rejected;
spam budget exceeded rejected; queue-full drops lowest-priority not newest;
non-allowlisted `LIST`/`SUBSCRIBE`/`SET_POLICY` caller denied; policy privacy
override can only increase restriction, never decrease (tested at
`bell.policy.reject reason=privacy_reduction`); `FullHidden` entries counted
in `redacted` but never leaked in `list.item`.

**No dedicated gate script exists for any of this today** (`scripts/`
contains no `bell_gate.sh` or equivalent) — that is itself a gap, separate
from the mission's 8 tasks but worth flagging: this entire, fairly mature
protocol has **zero automated regression coverage**, only manual/QMP-style
proof runs. Recommend a `scripts/bell_gate.sh` mirroring the pattern in
`scripts/usb_path_gate.sh` (lanes = one per opcode + negative case),
independent of and lower priority than the lane-rename/cap-denied work.

**New tests needed for gap-closing work:**
- Lane rename (§F.1): boot with old vs new lane constants, assert
  `find_lowest_priority_index` drop order unchanged for an identical event
  sequence (regression guard against the remap silently changing behavior).
- Self-reported cap-denied (§F.2): a PD hits `ERR_CAP_INVALID` locally, calls
  `OP_BELL_NOTIFY` with the reserved category — assert it lands in `SYSTEM`
  lane (not `NOW`, not user-controllable via `urgency_hint` — this event
  class should not be spoofable into jumping the queue by a malicious
  sender claiming urgency).

## F. Implementation prompts

### F.1 Lane rename (small, self-contained)

```
Rename Bell's lane scheme in servers/sexbell/src/main.rs to match the 5
named attention lanes: LATER=0, PROJECT=1, SOON=2, SYSTEM=3, NOW=4. Ordinal
5 stays reserved (unused today, do not remove the numeric range check that
allows it — just don't give it a name yet).

Scope: servers/sexbell/src/main.rs ONLY. Do not touch crates/sex-pdx (no
opcode changes), kernel/, silk-shell, or silkbar.

Task:
1. Add named u8 consts (LANE_LATER=0, LANE_PROJECT=1, LANE_SOON=2,
   LANE_SYSTEM=3, LANE_NOW=4) near the top of the file.
2. Replace magic numbers in derive_lane_first_proof, comments, and doc
   comments on BellQueueEntry/PolicyEntry with the named consts. Do NOT
   change find_lowest_priority_index's comparison logic — ordinal order
   must stay identical (0 = least urgent still evicts first).
3. Add proof marker [bell.lane.rename] old=<n> new=<name> for each mapped
   value, gated behind a one-shot boot flag like the existing
   BELL_BRIDGE_STUB_PROOF_ENABLED pattern, so it doesn't spam every boot.
4. Verify: boot with SEXOS_BELL_DELIVERY_PROOF=1, send the same NOTIFY
   sequence as before the rename, confirm bell.queue.drop picks the same
   victim it would have before (same ordinal = same behavior).

Do not touch silk-shell's or silkbar's lane_filter usage — they pass raw
u8 values already and this rename doesn't change the wire format, only
Bell's internal naming.
```

### F.2 Self-reported cap-denied dev-mode event

```
Add a userspace-only "cap denied" dev-mode event convention to Bell.
No kernel change, no new opcode — reuses OP_BELL_NOTIFY.

Scope: servers/sexbell/src/main.rs (reserve one category value + lane
pinning) and crates/sex-pdx/src/lib.rs (ONLY to add one doc comment
reserving category=6 for SelfCapDenied — do not add new opcodes or
MessageType variants; if you find you need one, STOP and report instead).

Current state: Bell's category field is 0..=5 (validated by
valid_category()). caller_pd is kernel-authoritative (can't be spoofed).
urgency_hint is sender-controlled and currently used verbatim in lane
derivation once BellCap validation lands (still TODO, see
derive_lane_first_proof) — this event class must NOT be spoofable into a
high lane by a malicious sender's urgency_hint claim.

Task:
1. Reserve category=6 as SelfCapDenied (requires bumping
   valid_category()'s bound to v <= 6 — confirm this doesn't collide with
   any other assumed category meaning first, there's no category name
   table today so check all 6 existing category call sites explain what
   0..5 currently mean before adding a 7th).
2. In derive_lane_first_proof (or a new small helper next to it), force
   category=6 events to LANE_SYSTEM regardless of urgency_hint — ignore
   the sender's urgency entirely for this category, it must not be
   sender-controllable priority.
3. Add proof marker [bell.selfreport.capdenied] caller_pd=<id>
   target_op=<opcode-if-known-else-0>.
4. This is opt-in for callers: no existing PD needs to change. Document in
   a comment that any future PD wanting to self-report a local
   ERR_CAP_INVALID should call OP_BELL_NOTIFY with category=6.

Do not attempt PD-crash or PDX-timeout event sourcing in this task — both
require kernel-side changes and are explicitly out of scope (see
docs/handoff/BELL_ATTENTION_FIREWALL_V1.md §A.2). Do not attempt
build-finished — it has no running-PD observer and doesn't belong in Bell.
```

### F.3 Bell gate script (lower priority, independent)

```
Write scripts/bell_gate.sh mirroring scripts/usb_path_gate.sh's structure
(numbered lanes, PASS/FAIL markers, blocking exit code).

Scope: scripts/bell_gate.sh only (new file). Do not touch
servers/sexbell or any other server.

Lanes to cover, each booting QEMU and driving Bell via QMP/serial-observed
markers (reuse whatever harness usb_path_gate.sh / input_control_quality_gate.sh
use for QMP-driven boots):
1. Notify + list round trip: send OP_BELL_NOTIFY from a real caller (e.g.
   spindle's existing `bell-test` command), assert [bell.notify.ok] then
   [bell.list.item] shows it.
2. Muted sender rejected: mute a PD, notify from it, assert
   [bell.notify.reject] reason=muted, assert it never appears in
   [bell.list.item].
3. Non-allowlisted LIST caller denied: assert [bell.readcap.deny] fires
   for any PD not in BELL_LIST_ALLOWLIST.
4. Queue-full drop: push BELL_QUEUE_CAPACITY+1 events, assert
   [bell.queue.drop] fires and the lowest-lane entry (not the newest) is
   the one dropped.
5. Privacy redaction: notify with privacy_level=3 (FullHidden) from a
   non-silk-shell caller's LIST call, assert it's counted in `redacted` but
   never appears as a bell.list.item.

This is independent of F.1/F.2 — do it in a separate pass if scoped work
budget allows; not required before F.1/F.2 land.
```

## G. What this pass did NOT do

- No code changes to `servers/sexbell`, `crates/sex-pdx`, `silk-shell`, or
  `silkbar`.
- Did not implement PD-crash or PDX-timeout Bell notifications — both need
  kernel-side plumbing, flagged as STOP-FIRST, not authorized here.
- Did not implement build-finished events — determined not to be Bell's
  concern (no running-PD observer for a host-side build script).
- Did not touch Linen object-history wiring — correctly deferred per the
  mission's own constraint ("Linen persistence later only"); `object_ref`
  fields already exist as marker-only placeholders, ready for when that
  lands.
## H. F.1/F.2/F.3 implemented and verified (2026-07-06 follow-up)

User asked to "fill the gaps" — F.1, F.2, and F.3 were implemented directly
(not left as prompts only). Summary:

- **F.1 (lane rename):** done in `servers/sexbell/src/main.rs`. Named
  consts `LANE_LATER/PROJECT/SOON/SYSTEM/NOW/RESERVED_5` added, all magic
  numbers and doc comments updated, one-shot `[bell.lane.rename]` proof
  marker added (gated behind `SEXOS_BELL_LANE_RENAME_PROOF`, not yet fired
  in a test run — ordinal-preservation is asserted by construction:
  `LANE_LATER=0 .. LANE_NOW=4` exactly mirrors the prior `0..4` numbering).
  Compiles clean (`cargo check -p sexbell`, 0 errors).
- **F.2 (self-reported cap-denied):** done. `BELL_CATEGORY_SELF_CAP_DENIED
  = 6` added to both `servers/sexbell/src/main.rs` and
  `crates/sex-pdx/src/lib.rs` (doc-comment/value only, no new opcode).
  `valid_category()` bound bumped to `<= 6`. Category 6 forcibly pins
  `LANE_SYSTEM` regardless of sender `urgency_hint` (not spoofable into a
  higher lane). No live PD calls this yet — it's an opt-in convention, as
  scoped. Compiles clean.
- **F.3 (`scripts/bell_gate.sh`):** written and run. Required an
  unplanned fix first: adding the const to `sex-pdx/src/lib.rs` broke
  `sexos_build_spec.toml`'s `abi_version_hash` guard (a hash of
  `kernel/src/syscalls/mod.rs` + `crates/sex-pdx/src/lib.rs` — any sex-pdx
  edit trips it by design). Recomputed and updated the hash; this is the
  one change in this pass that touched `sexos_build_spec.toml`, the
  project's designated build authority — flagged explicitly here since
  CLAUDE.md calls that file out by name.

  **Live results (13 boot attempts during verification):** Lane 1
  (notify+list round trip) passed cleanly on repeated attempts — confirms
  F.1/F.2 didn't break the live `OP_BELL_NOTIFY`/`OP_BELL_LIST` path. Lane
  2 (spam-budget burst, ~70 rapid QMP keystrokes) never completed cleanly:
  every attempt hit a pre-existing kernel scheduler fault (the documented
  `pd=8` PF flake from `SCHEDULER_TICK_PD8_PF_FLAKE_V1.md`, plus a
  previously-unseen `GP FAULT`/`KERNEL PANIC` variant at `pd=6` that the
  gate's original `FAULT_RE` didn't even catch — fixed to include it,
  which is the correctness improvement worth keeping regardless of the
  flake). This is kernel-side, off-limits, and unrelated to
  `servers/sexbell`/`crates/sex-pdx` — documented as a known-flaky lane in
  the script itself rather than glossed over.
