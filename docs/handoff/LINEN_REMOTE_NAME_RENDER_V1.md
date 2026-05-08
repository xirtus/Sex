# LINEN_REMOTE_NAME_RENDER_V1

Date: 2026-05-07
Status: LANDED
Requires: LINEN_PUBLIC_VIEW_SNAPSHOT_V1

## Files Changed

- `servers/silk-shell/src/main.rs` — 4 edits

No Linen PD changes. No kernel changes. No sex-pdx changes. No sexdisplay changes.
OP_LINEN_GET_PUBLIC_NAME (0x45) was already implemented in LINEN_PUBLIC_VIEW_SNAPSHOT_V1.

## Changes

### 1. `LinenObject` struct — add `name: [u8; 24]` and `name_len: u8`

```rust
struct LinenObject {
    ...
    display_name: &'static str,  // kept for seed objects
    name: [u8; 24],              // raw bytes from Linen PD (remote only)
    name_len: u8,                // 0 = use display_name; >0 = use name bytes
}
```

Seed objects: `name: [0u8; 24], name_len: 0` — continue using `display_name` static str.  
Remote objects: `name: <fetched bytes>, name_len: <actual len>`.

### 2. `LINEN_SEED_OBJECTS` const — add zero-init fields to all 6 entries

All 6 seed object literals updated with `name: [0u8; 24], name_len: 0`.  
No behavioral change for seed path.

### 3. `linen_fetch_remote_snapshot()` — fetch name bytes via OP_LINEN_GET_PUBLIC_NAME

After getting `(object_id, kind_byte, name_len)` from 0x44:
- Call 0x45 in 8-byte chunks: off=0, 8, 16 (max 3 calls per object)
- Each call: `pdx_call(SLOT_LINEN, 0x45, object_id, off, 8)` → `linen_sync_reply()`
- EOF = reply == 0. Error = reply as i64 < 0.
- Bytes sanitized: `if b >= 0x20 && b <= 0x7E { b } else { '?' }`
- Stored in `LinenObject.name[off..off+take]`, `name_len = fetched_len`

`sanitize_ascii(b: u8) -> u8` added as a pure fn (no heap, no alloc).

### 4. `linen_render_object_list()` — conditional name source in serial log

```rust
if obj.name_len > 0 {
    let name_str = core::str::from_utf8(&obj.name[..n]).unwrap_or("[bad_utf8]");
    serial_println!("[linen.object_list.row] ... name={} ...", name_str, ...);
} else {
    serial_println!("[linen.object_list.row] ... name={} ...", obj.display_name, ...);
}
```

`core::str::from_utf8` is safe on sanitized printable ASCII. No `from_utf8_unchecked`.

## Name Fetch Protocol

```
For each non-empty slot from 0x44:
  off = 0
  loop:
    pdx_call(SLOT_LINEN, 0x45, object_id, off, 8)
    reply = linen_sync_reply()
    if reply == 0: break (EOF)
    if reply as i64 < 0: error, abort fetch for this entry
    bytes = reply.to_le_bytes()
    take = min(name_len - off, 8)
    name[off..off+take] = sanitize_ascii(bytes[0..take])
    off += 8
```

Max 3 iterations per object (name_len ≤ 24, chunks of 8).
Total IPC calls per boot: 16 (snapshot slots) + up to 5×3 = 15 (name chunks) = 31 round-trips.

## Sanitization Rule

```
0x20..=0x7E → pass through (printable ASCII, space through tilde)
all other bytes → '?' (0x3F)
```

Applied byte-by-byte in `sanitize_ascii()`. No heap, no unchecked UTF-8.
`core::str::from_utf8` on the stored slice always succeeds (all bytes are valid 1-byte UTF-8).

## Proof Markers

Boot (first linen_paint_surface() → first Focus200):
```
[linen.remote.snapshot.begin]
[linen.snapshot.slot] slot=0 id=1 kind=0 name_len=12
[linen.remote.name.ok] id=1 len=12
[linen.remote.entry] slot=0 id=1 kind=0 name_len=12
[linen.snapshot.slot] slot=1 id=2 kind=0 name_len=10
[linen.remote.name.ok] id=2 len=10
[linen.remote.entry] slot=1 id=2 kind=0 name_len=10
[linen.snapshot.slot] slot=2 id=3 kind=0 name_len=10
[linen.remote.name.ok] id=3 len=10
[linen.remote.entry] slot=2 id=3 kind=0 name_len=10
[linen.snapshot.slot] slot=3 id=4 kind=1 name_len=8
[linen.remote.name.ok] id=4 len=8
[linen.remote.entry] slot=3 id=4 kind=1 name_len=8
[linen.snapshot.slot] slot=4 id=5 kind=0 name_len=13
[linen.remote.name.ok] id=5 len=13
[linen.remote.entry] slot=4 id=5 kind=0 name_len=13
[linen.remote.snapshot.ok] count=5
...
[linen.object_list.row] id=1 kind=Document state=Saved name=SexOS Kernel selected=true
[linen.object_list.row] id=2 kind=Document state=Saved name=Silk Shell selected=false
[linen.object_list.row] id=3 kind=Document state=Saved name=SexDisplay selected=false
[linen.object_list.row] id=4 kind=Project state=Saved name=Sessions selected=false
[linen.object_list.row] id=5 kind=Document state=Saved name=SexFiles Root selected=false
```

Names shown in serial log are real bytes from Linen PD SESSION, not seed placeholders.

## Limitations

### No on-screen text render
Surface 200 shows accent bars only — no pixel font, no text glyphs.
Proof is in serial output (name= in `[linen.object_list.row]` lines).

### No re-fetch
`LINEN_REMOTE_FETCHED = true` prevents re-fetch. SESSION changes after boot are not reflected until reboot.

### name_len capped at 24
`(name_len_raw as usize).min(24) as u8` — safe against malformed Linen replies.

### linen_sync_reply drops non-reply messages
During the 31-call fetch window, keyboard/mouse events arriving at shell are silently dropped.
This window is brief (Linen replies from in-memory SESSION with no blocking calls).

## Next Phase Recommendations

**LINEN_PUSH_INVALIDATE_PLAN_V1** (planning-only):  
Design how Linen notifies shell when SESSION changes (new object created, object removed).
Shell re-fetches snapshot on invalidation signal. Requires new opcode or existing slot.

**LINEN_OPEN_INTENT_STUB_V1**:  
When user presses Enter/Space on a selected Linen row, shell sends an OP_LINEN_OPEN
opcode to Linen PD (arg0=object_id). Linen logs `[linen.open.intent]` and returns 0 (stub).
Proves the selection→intent→handler pipeline without implementing actual open behavior.
