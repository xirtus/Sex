# E9 Blocker Fix: CRITICAL-1 + MEDIUM-3

**Status:** Complete  
**Phase:** E9 pre-durable-storage gate — blocking fixes  
**Files changed:** `servers/sexstore/src/main.rs`, `servers/silk-shell/src/main.rs`  
**Build:** `[SEXOS ENTRYPOINT] success`

---

## CRITICAL-1 — REPLY_STATUS_BIT bit-63 collision

### Root Cause

`REPLY_STATUS_BIT = 0x8000_0000_0000_0000` uses bit 63 to distinguish status replies
from GET value replies. Stored u64 values are returned raw on GET. For key 0x01
(scene settings blob), byte 7 (bits 56-63 LE) holds the XOR checksum of bytes 0-6.
Checksum `b[0]^...^b[6]` can be ≥ 0x80, setting bit 63 of the stored u64. A valid
accepted value would then have bit 63 set, and the shell's GET reply handler would
misclassify it as a status reply (KV_NOT_FOUND, etc.) instead of returning the data.

### Fix — `servers/sexstore/src/main.rs` (`store_validate_value`)

Added bit-63 guard at entry:
```rust
if value & REPLY_STATUS_BIT != 0 { return false; }
```

Added checksum mask: XOR checksum now validated against `expected & 0x7F`:
```rust
let chk = (b[0] ^ b[1] ^ b[2] ^ b[3] ^ b[4] ^ b[5] ^ b[6]) & 0x7F;
if b[7] != chk { return false; }
```

### Fix — `servers/silk-shell/src/main.rs` (`pack_scene_settings_blob` / `unpack_scene_settings_blob`)

`pack_scene_settings_blob`: checksum byte capped to 7 bits before storing:
```rust
let chk = (b[0] ^ b[1] ^ b[2] ^ b[3] ^ b[4] ^ b[5] ^ b[6]) & 0x7F;
```

`unpack_scene_settings_blob`: verifies against masked expected:
```rust
let expected = (b[0] ^ b[1] ^ b[2] ^ b[3] ^ b[4] ^ b[5] ^ b[6]) & 0x7F;
if b[7] != expected { return None; }
```

Both sides now agree: bit 63 of any valid stored value is always 0.

### Shell reply handler — `handle_sexstore_get_reply`

Added status-first dispatch using new helpers:
- `store_reply_is_status(reply)` — checks bit 63
- `store_reply_status(reply)` — strips bit 63 for code extraction
- `store_reply_is_value(reply)` — affirms bit 63 clear

GET reply path:
1. If bit 63 set → log `[shell.store.reply.status]` with named code, return
2. Else → pass u64 to `unpack_scene_settings_blob()` as before

---

## MEDIUM-3 — Esc double-dispatch (panel close + AccessZoomToggle)

### Root Cause

When `SCENE_SETTINGS_ACTIVE` and Esc (scancode 0x01) pressed:
1. Panel close block fires: deactivates surface, sets `SCENE_SETTINGS_ACTIVE = false`
2. Execution continues — NO `continue` or consumed flag
3. `scancode_to_action(0x01)` returns `Some(AccessZoomToggle)` → Atlas toggle fires

Result: panel closes AND Atlas mode activates simultaneously.

### Fix — `servers/silk-shell/src/main.rs` (HID key dispatch ~line 8114)

Added `let mut panel_consumed = false;` before the `SCENE_SETTINGS_ACTIVE` block.

Set `panel_consumed = true` in all four matched arms (0x01, 0x02, 0x03, 0x04).

Restructured downstream dispatch:
```rust
if panel_consumed {
    // panel handled key; skip Atlas and action dispatch
} else if ATLAS_MODE_ENABLED && scancode != 0x44 {
    handle_atlas_keyboard(scancode);
    mutated = true;
} else if let Some(action) = scancode_to_action(scancode) {
    // ... normal action dispatch
}
```

### Behavioral invariants preserved

| Scenario | Before | After |
|----------|--------|-------|
| Panel open + Esc | close panel + AccessZoomToggle ❌ | close panel only ✅ |
| Panel open + Key 1/2/3 | panel cmd + possible action ❌ | panel cmd only ✅ |
| Panel open + other key | pass through ✅ | pass through ✅ |
| Atlas active + any non-F10 | Atlas handles ✅ | Atlas handles ✅ |
| Normal mode + Esc | AccessZoomToggle ✅ | AccessZoomToggle ✅ |
| Normal mode + F7 | ToggleSceneSettings ✅ | ToggleSceneSettings ✅ |

Atlas Esc behavior unchanged — when panel is closed (SCENE_SETTINGS_ACTIVE=false),
`panel_consumed` stays false and the full Atlas/action path runs normally.

---

## E9 Gate Status After These Fixes

| Finding | Severity | Status |
|---------|----------|--------|
| REPLY_STATUS_BIT bit-63 collision | CRITICAL | **Fixed** |
| Esc double-dispatch | MEDIUM | **Fixed** |
| No shell-side status protocol client | HIGH | Fixed (separate HIGH-1 commit) |

CRITICAL-1 and MEDIUM-3 both resolved. HIGH-1 resolved via `handle_sexstore_get_reply`
rewrite and `STORE_REPLY_STATUS_BIT`/`STORE_KV_*` constants in silk-shell.

E9 gate: all three blockers cleared. Safe to proceed with durable storage integration
after E9_PRE_DURABLE_STORAGE_SAFETY_AUDIT_V1 re-run.
