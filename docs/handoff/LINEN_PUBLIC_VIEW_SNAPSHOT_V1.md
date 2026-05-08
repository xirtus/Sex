# LINEN_PUBLIC_VIEW_SNAPSHOT_V1

Date: 2026-05-07
Status: LANDED
Requires: LINEN_SEXFILES_READBACK_V1

## Files Changed

- `servers/linen/src/session.rs` — add `get_at_slot()` method
- `servers/linen/src/main.rs` — 3 edits (constants, dispatch, 2 handlers)
- `servers/silk-shell/src/main.rs` — 4 edits (constants, flag, fetch fn, paint-surface update)

No kernel changes. No sex-pdx changes. No sexfiles changes. No sexdisplay changes.

## Architecture

Linen PD exposes two new public opcodes. Shell calls them on first `linen_paint_surface()` invocation, populates `LINEN_OBJECTS` from Linen's real SESSION, then renders via the existing `linen_render_object_list()` pipeline unchanged.

SESSION.list owner filter is preserved — these new opcodes are server-internal: they bypass the filter at the Linen server level, not at the kernel IPC level.

## New Opcodes

```
OP_LINEN_GET_PUBLIC_SNAPSHOT: u64 = 0x44
  arg0 = slot_idx (0..16)
  reply = 0 if slot empty
  reply = (object_id & 0xFFFF_FFFF) | (kind << 32) | (name_len << 40)

OP_LINEN_GET_PUBLIC_NAME: u64 = 0x45
  arg0 = object_id, arg1 = byte_offset, arg2 = max_len (≤8)
  reply = up to 8 name bytes LE-packed. 0 = EOF.
```

## Session.rs Change

```rust
pub fn get_at_slot(&self, idx: usize) -> Option<LinenObject> {
    self.objects.get(idx).and_then(|s| *s)
}
```

Called by `handle_get_public_snapshot` to access exact slot position without the "first match AT OR AFTER" semantics of `list()`. Required for correct sequential enumeration by the shell.

## Shell Fetch Protocol

`linen_fetch_remote_snapshot()` iterates slot_idx 0..16:
1. Fire `pdx_call(SLOT_LINEN, OP_LINEN_GET_PUBLIC_SNAPSHOT, slot_idx, 0, 0)`
2. `linen_sync_reply()` — spin on pdx_listen_raw(0), return first type_id==0x1 reply
3. If reply==0: skip (empty slot). If reply!=0: parse object_id/kind_byte/name_len
4. Map kind_byte → LinenObjectKind (0→Document, 1→Project, else→Document)
5. Populate `LINEN_OBJECTS[write_idx]` with `display_name: "[linen.remote]"`

Name bytes not fetched in V1 (display_name is &'static str, no text rendering path).
OP_LINEN_GET_PUBLIC_NAME is implemented and functional — reserved for a future text-render phase.

## One-Shot Fetch

Gated by `static mut LINEN_REMOTE_FETCHED: bool = false`.
`linen_paint_surface()` checks this flag and calls `linen_fetch_remote_snapshot()` on first invocation only. Subsequent paints use the cached LINEN_OBJECTS contents.

## IPC Safety

`linen_sync_reply()` drops non-reply messages (type_id != 0x1) during the fetch window.
Fetch is brief (16 synchronous round-trips, Linen replies from memory, no sexfiles calls).
Risk: a keyboard event arriving during fetch is silently dropped. Acceptable for proof phase.

## Kind Mapping (Linen → Shell)

| Linen ObjectKind | kind_byte | Shell LinenObjectKind |
|---|---|---|
| Document | 0 | Document (1) |
| Session | 1 | Project (0) |
| Unknown | 2 | Document (1) |

## Preserved Invariants

- `SESSION.list(caller_pd, idx)` owner filter unchanged — shell still cannot call list/get on Linen's objects via the old opcodes
- Linen is sole data authority — new opcodes are opt-in exports controlled by Linen
- Shell is sole painter of surface 200 — no display bypass
- No new kernel, sex-pdx, or sexfiles changes

## Proof Markers

Boot sequence (first paint of surface 200):
```
[linen.remote.snapshot.begin]
[linen.snapshot.slot] slot=0 id=1 kind=0 name_len=12
[linen.remote.entry] slot=0 id=1 kind=0 name_len=12
[linen.snapshot.slot] slot=1 id=2 kind=0 name_len=10
[linen.remote.entry] slot=1 id=2 kind=0 name_len=10
[linen.snapshot.slot] slot=2 id=3 kind=0 name_len=10
[linen.remote.entry] slot=2 id=3 kind=0 name_len=10
[linen.snapshot.slot] slot=3 id=4 kind=1 name_len=8
[linen.remote.entry] slot=3 id=4 kind=1 name_len=8
[linen.snapshot.slot] slot=4 id=5 kind=0 name_len=13
[linen.remote.entry] slot=4 id=5 kind=0 name_len=13
[linen.remote.snapshot.ok] count=5
[linen.object_list.render] ...
[linen.object_list.row] id=1 kind=Document ... selected=true
...
```

## Remaining Blockers

### display_name placeholder
Remote entries show `display_name="[linen.remote]"` in serial logs.
Real name display requires either:
- Storing name bytes in LinenObject (changes struct — `display_name: [u8; 24]` instead of `&'static str`)
- Or a small text-render primitive that reads name bytes from a side table

### Fetch on paint vs. fetch on demand
Current: fetch on first linen_paint_surface(). Re-fetch not possible without reboot.
Future: add a LINEN_REMOTE_STALE flag; re-fetch on explicit trigger (e.g., long-press, or Linen pushes an invalidation message).

## Next Phase Recommendation

**LINEN_REMOTE_NAME_RENDER_V1** — store name bytes in shell's object record, render first N bytes as pixel font or hex-encoded label. Requires either changing `LinenObject.display_name` type or a parallel name table.

Alternative: **LINEN_PUSH_INVALIDATE_V1** — Linen sends a push notification to shell when SESSION changes, shell re-fetches. Currently there is no way for Linen to initiate contact with shell (SLOT_SHELL exists but shell has no handler for type_id 0x44/0x45 from Linen).
