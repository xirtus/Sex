# BELL_EVENT_FILTER_KEYBOARD_V1

Status: implemented (proof-local, ring-based)

Changes:
- Added default-off Bell filter proof markers over local Bell ring state.
- Uses existing row navigation helper; no notification model redesign.

Markers:
- [bell.filter.source] source=NAME count=N ok=N
- [bell.filter.nav] old=N new=N ok=N
- [bell.filter.proof.done] ok=N
