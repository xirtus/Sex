# SEXOS_BIG_APP_SPRINT_V2

## OVERALL: PASS

Sprint executed in waves on top of same-day work (LINEN_OWN_CONTENT_SID_V1,
QUIL_TEXT_BUFFER_STUB_V1, MESH_READONLY_PD_GRAPH_V1, FOCUS_NAV_LIVE_V1,
APP_DATA_LAYER_V1 — see those handoffs). Shell-side only; kernel/sex-pdx/
sexdisplay untouched.

## WAVE 1 — Frozen product gate: `scripts/app_data_gate.sh`

One boot, 24 rows, **[appdata.gate.result] PASS**:
spindle visible+typing · linen visible sid 157 + objects + j/k select ·
quil visible + typing + save/load roundtrip · mesh visible + focus-change
refresh · collar core-app allow + grant match + grants list render ·
linen→quil open · bell event + lane text + **multi-entry ring (total=2)** ·
fault_free · auth_free · rsp0 · pixel_quil + pixel_shell_text screendump
scans. `SKIP_BUILD=1` supported; QMP socket path must stay <108 bytes.

## WAVE 2 — Features landed this sprint

| Feature | Detail | Marker |
|---|---|---|
| Collar grants list | Real text on shell-owned sid 203: header count + up to 5 active-grant rows, `>` selection; renders at open + each j/k nav | `[collar.grants.render.ok] grants=12 ok=1` |
| Bell lane text | Count + latest OBJ/BUF + selected row on sid 204; appended to `bell_render_event_list` (fires on nav/ring change) | `[bell.lane.render.ok] total=2 ok=1` |
| Linen j/k drain parity | SelectNext/Prev + OpenObjectInQuil arms added to drain broad dispatch (were main-path-only) | `[linen.nav.select.ok] object=N path=drain`, `[linen.quil.open.ok]` |
| **Bell reuse-path bug fix** | `open_linen_object_in_quil` passed `dynamic_buffer_id` to the bell event even when the reuse path linked a SEED buffer with a different id → bell cross-check rejected (`buffer_valid=false`) → ring silently never grew on reuse. Now tracks `linked_buffer_id` | `[bell.ring.write]` on reuse opens |

Already done pre-sprint (not repeated): linen own sid, spindle 40x12 grid,
quil editing/save/load, mesh PD graph + refresh, palette routes for all
apps, focus-nav for bell/collar/atlas.

## WAVE 3 — Product flow (inside the gate)

Scroll Lock → type `help`⏎ (spindle) → F12 (mesh) → FocusLinen → j →
Enter (open #1) → refocus → j → Enter (open #2) → FocusBell → j/j (nav
total=2) → FocusCollar → j/k (grants nav) → Esc+PgDn (minimize both) →
FocusQuil → Esc → type → Save → Load → screendump. Zero faults, zero AUTH
across the whole drive.

## WAVE 4 — Deferred (by design, docs only)

WebStub / networking / sext pager / tuxedo / sexgemini / USB kbd protocol
work / sexdisplay text-model extension. See STUB_SERVER_KILL_LIST_V1 for
the ranked stub audit (sext = only stub with a live caller).

## Proof table (this sprint's runs)

```
entrypoint_build.sh                     PASS (x3)
scripts/app_data_gate.sh                PASS 24/24 rows
scripts/usb_path_gate.sh                PASS 8/8 rows
rsp0_regression_gate.sh (in-gate)       PASS
fault scan (all lanes)                  0 #PF/#GP/panic/fault.kill
AUTH rejects (all lanes)                0
```

## Files changed (backups `.bak.big_sprint_v2`, `.bak.linen_quil_data_v1`)

- `servers/silk-shell/src/main.rs` — collar grants text, bell lane text,
  linen drain-nav parity, linked_buffer_id fix (+ same-day: seed fallback,
  core-app cap allowlist, mesh focus refresh, focus-nav passthroughs).
- `scripts/app_data_gate.sh` — NEW frozen gate.

## Known limits / notes

- Quil content sid 156 partially occluded by the shell's quil buffer-list
  overlay on frame 201 at gate-end; pixel_quil row is a nonzero-evidence
  check (glyphs on screen), full typing proof is marker rows. Follow-on:
  move shell buffer-list drawing or z-order tweak (STOP FIRST: z policy).
- Bell/collar drain nav acts on both key edges (pre-existing Mesh pattern).
- `claude-references/` dir referenced by CLAUDE.md does not exist —
  handoffs in docs/handoff are the real memory.

## Recommended commits (small, reversible)

1. `shell: linen own content sid 157` (linen/main.rs)
2. `shell: quil live typing + reserved-key passthrough` (quil + shell)
3. `shell: focus-nav live for bell/atlas/collar/linen-enter`
4. `shell: app data layer — seed fallback, core-app caps, mesh refresh`
5. `shell: collar grants + bell lane text, linen drain nav, bell reuse fix`
6. `gate: add frozen app_data_gate` (+ this handoff + lane handoffs)

## Changelog

- 2026-07-18: sprint complete — product gate frozen, collar/bell visible
  data lists, linen drain nav, bell reuse-path ring fix. 24/24 + 8/8.
