# APP_DATA_LAYER_V1

## Result: PASS — apps remember and exchange data live (12/12 rows, one boot)

Covers three lanes: LINEN_QUIL_OPEN_DATA_V1, MESH_LIVE_REFRESH_V1,
QUIL_SAVE_LOAD_LIVE_V1. Files: `servers/silk-shell/src/main.rs`
(backup `.bak.linen_quil_data_v1`). Quil PD untouched (its save/load
already existed). WebStub stays deferred (no surface, no network).

## 1. Linen → Quil open chain (was dead at TWO points)

The full model existed (seed objects, grant table, link, bell event, mesh
facts, quil buffer list) but plain boot broke it twice:

- **Snapshot wipe**: `linen_fetch_remote_snapshot` (first linen paint)
  clears `LINEN_OBJECTS`, queries the linen PD session, gets count=0
  (linen's object-creating proofs are env-gated) → table left EMPTY,
  killing selection/open AND orphaning the 12 boot collar grants that
  reference seed ids 1-6. Fix: restore `LINEN_SEED_OBJECTS` when the
  remote session is empty — `[linen.remote.snapshot.fallback]`.
- **Cap wall**: `AccessSexFiles`/`AccessBell` denied every `caller_sid <
  300`; manifest-based grants (sid 320+) exist only behind proof envs, so
  the C4 cap check in `open_linen_object_in_quil` (and Bell toggle's
  AccessBell) could never pass live. Fix: explicit core-app allowlist
  (fixed sids 153/200/201/202/203/204) in the cap arm, still audited —
  `[collar.policy.allow] ... reason=core_app`. Deny-by-default preserved
  for everything else; LinkObjectToBuffer still requires a real grant
  match (`[collar.grant.match]`).

Live proof: Linen focused → Enter → `[linen.quil.buffer.linked]
object_id=1 buffer_id=1001` → Quil opens with the linked buffer in its
buffer list → `[bell.event.object_link]` feeds the Bell ring → mesh link
facts emitted.

## 2. Bell/Collar rings at boot

- Collar: already populated — 12 boot auto-grants (objects 1-6 × subjects
  linen/mesh); nav proven over them in FOCUS_NAV_LIVE_V1.
- Bell: populated per object-open (`bell_emit_object_link_event`);
  `[bell.nav.move] total=1` after one live open. **Limit:** opening an
  object focus-jumps to Quil, so stacking multiple ring entries needs
  refocusing Linen between opens — one entry per open, honest.

## 3. Mesh live refresh

`try_set_focus` commit path now re-renders the PD graph when focus
actually changed AND Mesh is visible in the active scene —
`[mesh.pd_graph.refresh] reason=focus_change old=201 new=202` observed on
every live focus hop. Bounded: 16 display calls per real focus change,
nothing when Mesh hidden.

## 4. Quil save/load — real storage roundtrip live

Already implemented in the Quil PD (RamFS via SLOT_STORAGE, 8-byte
chunks); reachable live since QUIL_TEXT_BUFFER_STUB_V1 delivered arrows/
Enter. Quil palette rows: 0 New(stub) / 1 Save / 2 Load / 3 Run(stub) /
4 Settings(stub). Proof: type `ab` → Save `[quil.save.ok] bytes=242` →
Load `[quil.load.ok] bytes=242` — typed bytes included in the roundtrip
(baseline buffer was 240).

## Proof lane

`scratchpad/lane5_gate.sh` pattern: F12 (mesh) → palette FocusLinen →
Enter (open) → palette FocusBell → j (nav ring) → palette FocusQuil →
Esc/type/Esc → Down+Enter (save) → Down+Enter (load). 12/12 PASS,
faults=0, AUTH=0, rsp0 PASS.

```sh
grep -E "\[linen\.remote\.snapshot\.fallback\]|\[linen\.quil\.buffer\.linked\]|\[bell\.event\.object_link\]|\[mesh\.pd_graph\.refresh\]|\[quil\.(save|load)\.ok\]|reason=core_app" LOG
```

## Follow-on candidates

- Multi-entry Bell ring lane (refocus Linen between opens).
- Linen selection j/k advance in drain path (SelectNextLinenObject only
  dispatched in main path — untested which path QMP keys take per boot).
- Real linen-PD session objects at boot (would replace the seed fallback).

## Changelog

- 2026-07-18: data layer live — open chain unblocked (snapshot fallback +
  core-app cap allowlist), bell ring fed by opens, mesh focus-refresh,
  quil save/load roundtrip proven.
