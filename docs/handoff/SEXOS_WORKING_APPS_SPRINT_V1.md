# SEXOS_WORKING_APPS_SPRINT_V1

## Result: PASS (Spindle visible + typed input renders; pre-existing scheduler flake unchanged)

## What works now, user-visible

- **Spindle terminal renders real pixels for the first time ever.** The
  input-connected `apps/spindle` PD (pd=12) now owns a visible content
  surface (sid **154 / 0x9A**, navy panel, bright glyph text) drawn through
  the proven compositor route: `0xEC` surface upsert + `0xFA` text clear +
  `0xFB` packed text draw. No legacy `OP_WINDOW_CREATE` pointer ABI, no
  sexdisplay/kernel/sex-pdx changes.
- **Typed characters appear on screen.** Prompt row shows `> ` + live line
  tail; Enter pushes to history + scrollback (command output shows in the
  5-row scrollback tail); Backspace visibly edits.
- **Ghost autosuggest + Tab/Right accept + history recall still work**
  (`[spindle.ghost.accept] len=2` reproduced live this run).
- **Launch path:** Scroll Lock (ToggleSpindle) focuses Spindle and routes
  keys to it — proven live. Command palette idx 0 = FocusSpindle (no
  `ionshell` exists anywhere; nothing to alias). New marker
  `[spindle.launch.route.ok]` emitted on palette FocusSpindle success.
- **App placeholders (Phase 4, pre-existing, verified in boot logs):**
  Linen (sid 200) is boot-tiled visible; shell's Spindle frame (sid 153)
  boot-tiled at 640,385 640x335 (the new content surface lands inside it);
  Quil/Mesh/Collar/Bell are palette-openable placeholders already
  implemented shell-side. No new placeholder code added — shell edits are
  restricted, and they already exist.

## Root cause fixed

`apps/spindle` has never rendered because its only render path used the
legacy pointer-struct `OP_WINDOW_CREATE` (0xE4) ABI, which sexdisplay's
handler decodes as w=0 → drops without reply → caller hangs (see
SPINDLE_LIVE_TERMINAL_GHOST_HISTORY_V1.md §A). Fix: bypass 0xE4 entirely.
Spindle now creates its **own** surface via `0xEC` (binding `owner_pd` to
itself, so sexdisplay's per-op ownership checks pass — shell-owned sid 153
is untouchable by other PDs) and draws a 6-row × 20-col text grid via
`0xFA`/`0xFB`, the same primitives kaleidoscope (sid 300) uses live.

Key implementation facts:

- sexdisplay text model: 128-byte `text_buf` per surface, wraps at 20
  chars/line, 5×7 glyphs (0x20–0x5A; lowercase auto-mapped to uppercase),
  text inset x+8/y+24. 120 of 128 bytes used (6 rows).
- **Serial-spam dodge:** sexdisplay logs `[sexdisplay.text.draw]` on every
  0xFB while `text_len <= 32`. `content_flush()` sends the 15 8-byte
  chunks **highest offset first**, so `text_len` jumps to 120 on the first
  write and the diagnostic never fires. Do not "fix" the chunk order to
  ascending.
- Geometry: sid 154 at x=1072 y=660 w=200 h=104 — bottom-right, inside
  the boot-tiled shell Spindle frame region.
- Render triggered once at boot (banner `SPINDLE TERMINAL` / `TYPE HELP`)
  and after every processed key-down. ~16 pdx_calls per keystroke, no
  per-key serial output (echo marker budgeted at 16, frame marker at 8).

## Files changed

- `apps/spindle/src/main.rs` (backup: `main.rs.bak.visible_surface_v1`):
  content-surface consts + `content_flush()` + `content_render()`; surface
  create + banner + initial render before `[spindle.ready]`; budgeted
  `[spindle.input.echo.ok]` + `content_render` after `handle_key` in main
  loop. Gotcha: consts must sit **above** `_start`'s `#[no_mangle]`
  attribute or rustc errors (`const items should never be #[no_mangle]`).
- `servers/silk-shell/src/main.rs` (backup:
  `main.rs.bak.working_apps_sprint_v1`): one line — `[spindle.launch.route.
  ok]` marker in `Command::FocusSpindle` exec branch (launcher route only).

## Proof runs (3 boots + 3 palette boots, QMP PS/2 keyboard)

Lane script: boot → wait `[spindle.ready]` → scroll_lock, a, b, c, ret,
a, right, backspace → screendump → fault scan → pixel scan of sid-154
region (x 1072–1272, y 660–764).

- **Run 1 (definitive PASS):** every marker row PASS —
  `surface.create.begin/ok sid=154 status=0`, `render.frame.ok`,
  `input.echo.ok`, `key.recv`, `line.append` a/b/c, `history.push`,
  `ghost.accept len=2`. Pixel scan: 2145 navy bg + 55 bright glyph px in
  the sid-154 region. One `KERNEL PAGE FAULT` on the **final log line**,
  after the full sequence + screendump completed.
- Run 2: flake fired before keystrokes; banner still visible (236 glyph
  px). Run 3: zero faults but silent input stall (known flake variant).
- Palette lanes: `[command_palette.open]` reached once, then stall/fault
  before Enter each time — palette-under-QMP flakiness already documented
  in SPINDLE_LIVE_TERMINAL_GHOST_HISTORY_V1 §E2. `[spindle.launch.route.
  ok]` is compile-verified on the already-live-proven
  `[shell.palette.focus.result] target=SPINDLE` path.

### Fault scan verdict

The only fault in any run is the **pre-existing** Scheduler::tick pd=8 PF
flake (SCHEDULER_TICK_PD8_PF_FLAKE_V1.md): this build's signature
`addr=0x68 rip=0xffffffff802005bc rsp=0x4444446804c0 err=0x0 pd=8`; RIP
symbolizes inside `Scheduler::tick` (0xffffffff80200130 + 0x48c). Offsets
shifted vs the handoff's recorded signature only because the kernel was
rebuilt. Kernel untouched by this sprint. No AUTH ownership rejects, no
new serial spam (logs ~3.6–4.9k lines/boot, unchanged scale).

## Marker grep commands

```sh
grep -E "spindle\.surface\.create\.(begin|ok)" LOG
grep -E "spindle\.render\.frame\.ok" LOG          # sid=154 cols=20 rows=6
grep -E "spindle\.input\.echo\.ok" LOG
grep -E "spindle\.ghost\.accept" LOG
grep -E "spindle\.launch\.route\.ok" LOG          # palette FocusSpindle
grep -E "KERNEL PAGE FAULT|DOUBLE FAULT|panic|fault\.kill" LOG
```

Lane scripts (session scratchpad, copy if needed):
`spindle_visible_proof.sh`, `palette_route_proof.sh` — boot + QMP drive +
screendump pixel check. Short `GATE_DIR` required (QMP unix socket path
< 108 bytes).

## Remaining blockers / next work

1. **Scheduler::tick pd=8 PF flake** still kills ~2/3 of QMP input lanes
   (fault or silent stall). Biggest single obstacle to reliable runtime
   proofs. Kernel-side, needs its own STOP-FIRST pass
   (SCHEDULER_TICK_PD8_PF_FLAKE_V1.md phase-2: Task clobber + IRET frame
   off-by-8 suspect).
2. Spindle content surface is fixed-geometry; it does not follow shell
   retiling/hide of frame sid 153. Options: shell forwards geometry via
   existing opcode on toggle (needs scoped design), or accept fixed panel.
3. 20×6 text grid is the ceiling of sexdisplay's current per-surface text
   model (128 bytes). A real terminal grid needs a compositor text-model
   extension — STOP FIRST (sexdisplay ABI).
4. Ghost suffix not drawn on the visible surface (single `text_color` per
   surface — no dim color mixing possible). Logic + markers intact.
5. Dead `YarnSession` in silk-shell still unreferenced by real input
   (STUB_SERVER_KILL_LIST_V1 candidate).

## Changelog

- 2026-07-18: First-ever visible Spindle pixels + live typed-character
  rendering, app-side only (sid 154 self-owned surface, 0xEC/0xFA/0xFB).
  Launcher marker added. Verified live via QMP with screendump pixel scan.
