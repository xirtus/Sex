# APP_LAUNCHER_VISUAL_KEYS_HELP_V1

Status: implemented (marker-first, no layout redesign)

Changes:
- Added default-off proof gate support for launcher help markers.
- Emits:
  - [launcher.help.keys] key=NAME action=NAME
  - [launcher.help.row] idx=N app=NAME key=NAME status=NAME
  - [launcher.help.proof.done] ok=N

Notes:
- Scope stayed in `servers/silk-shell/src/main.rs` only.
- No kernel/ABI/USB/pointer changes.
