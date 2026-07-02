# E9 Blocker Fix: HIGH-1 — Shell Storage Status Protocol Client

**Status:** Complete  
**Phase:** E9 pre-durable-storage gate — HIGH-1  
**Files changed:** `servers/silk-shell/src/main.rs`  
**Build:** `[SEXOS ENTRYPOINT] success`

---

## Problem (from TODAY_SAFETY_AUDIT)

E* storage changes (E4-E7) added server-side status encoding in `servers/sexstore/src/main.rs`:
- `REPLY_STATUS_BIT = 0x8000_0000_0000_0000` discriminates status from value
- Status codes: `KV_OK=0, KV_NOT_FOUND=1, KV_FULL=2, KV_INVALID_KEY=3, KV_INVALID_VALUE=4, KV_DENIED=5`

But `servers/silk-shell/src/main.rs` had zero matches for any of these constants. The old
`handle_sexstore_get_reply()` treated `value == 0` as "not found" (pre-E* protocol). Under
the new protocol, 0 is a valid GET success result (stored val = 0), and "not found" is
`REPLY_STATUS_BIT | KV_NOT_FOUND`. The server+client were out of sync.

---

## Fix

### New shell-side constants (mirrors sexstore, avoids cross-PD import)

```rust
const STORE_REPLY_STATUS_BIT: u64 = 0x8000_0000_0000_0000;
const STORE_KV_OK:            u64 = 0x00;
const STORE_KV_NOT_FOUND:     u64 = 0x01;
const STORE_KV_FULL:          u64 = 0x02;
const STORE_KV_INVALID_KEY:   u64 = 0x03;
const STORE_KV_INVALID_VALUE: u64 = 0x04;
const STORE_KV_DENIED:        u64 = 0x05;
```

### New helpers

```rust
fn store_reply_is_status(reply: u64) -> bool { reply & STORE_REPLY_STATUS_BIT != 0 }
fn store_reply_status(reply: u64)    -> u64  { reply & !STORE_REPLY_STATUS_BIT }
fn store_reply_is_value(reply: u64)  -> bool { reply & STORE_REPLY_STATUS_BIT == 0 }
```

### Updated `handle_sexstore_get_reply()`

New dispatch order:
1. **Status path** (bit 63 set): extract code, log named status, apply defaults if
   `KV_NOT_FOUND` (scene settings not yet persisted = expected on first boot), log
   `[shell.store.default] reason=status_reply`.
2. **Value path** (bit 63 clear): pass to `unpack_scene_settings_blob()` as before.

Named status log: `[shell.store.reply.status] code={} name={}` where name is one of
`ok/not_found/full/invalid_key/invalid_value/denied/unknown`.

### Proof markers added

| Marker | Condition |
|--------|-----------|
| `[shell.store.reply.status] code=N name=X` | Any status reply from sexstore |
| `[shell.store.reply.value] key=0x01` | Value reply (data path) |
| `[shell.store.default] reason=status_reply` | Defaults applied on any status |

---

## Protocol Symmetry (post-fix)

| sexstore sends | silk-shell receives | Correct? |
|----------------|---------------------|----------|
| `REPLY_STATUS_BIT \| KV_NOT_FOUND` | `store_reply_is_status` → log + default | ✅ |
| `REPLY_STATUS_BIT \| KV_DENIED` | `store_reply_is_status` → log + default | ✅ |
| `REPLY_STATUS_BIT \| KV_INVALID_KEY` | `store_reply_is_status` → log + default | ✅ |
| raw u64 value (bit 63 = 0) | `store_reply_is_value` → unpack | ✅ |
| `0` (was old "not found" sentinel) | treated as valid value (bit 63=0) → unpack | ✅* |

*`unpack_scene_settings_blob(0)` will fail magic check (byte 0 != 0xAC) and return
`None`, triggering the existing corrupt-blob fallback. Graceful degradation.

---

## Why Shell-Local Constants, Not Cross-PD Import

sexstore status constants are declared locally in `servers/silk-shell/src/main.rs` with
`STORE_` prefix, not imported from `sex-pdx` or `servers/sexstore`. This matches
the shell-local namespace policy: the protocol wire format (REPLY_STATUS_BIT scheme)
is stable once both sides agree, but crossing PD boundaries via import would couple
build artifacts and require `sex-pdx` ABI changes. `STOP FIRST` not triggered.
Values must stay in sync manually; the `STORE_` prefix prefix makes the pairing explicit.
