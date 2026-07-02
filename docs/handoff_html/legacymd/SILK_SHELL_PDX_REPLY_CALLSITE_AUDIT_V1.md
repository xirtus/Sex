# silk-shell pdx_reply Callsite Audit V1

## Status: Patched — 3 dead callsites removed

---

## Callsite Inventory

| # | Opcode | Line (before) | Caller | Caller Waits? | Classification |
|---|--------|---------------|--------|---------------|---------------|
| 1 | `OP_SHELL_BIND_BUFFER` (0x14) | 9780 | sexdrive (`apps/sexdrive`) | **No** — `pdx_call` then immediate render loop | Dead/stale |
| 2 | `OP_USB_MOUSE_REPORT` (0x260) | 9885 | sexinput (`servers/sexinput`) | **No** — `pdx_call_checked` discards result | Dead/stale |
| 3 | `OP_SCENE_SETTINGS_CMD` (0xFB) | 10966 | **No sender exists** — defined only in silk-shell | N/A | Dead/stale |

All three shared the same pattern:
- `pdx_reply(0)` — target_pd=0 (invalid), syscall 1 (unhandled), no val — fully broken
- No caller listens for a reply after sending

---

## Classification Details

### Callsite #1: OP_SHELL_BIND_BUFFER
- **Sender**: `apps/sexdrive/src/main.rs` line 117
- **Sender code**: `pdx_call(SLOT_SHELL, OP_SHELL_BIND_BUFFER, shared_addr, 0, 0);`
- **After call**: Immediately enters render loop (`for y in 0..768 { ... }`)
- **No listen/reply wait**: None. Fire-and-forget.
- **Verdict**: Dead. The reply was never consumed.

### Callsite #2: OP_USB_MOUSE_REPORT
- **Sender**: `servers/sexinput/src/main.rs` lines 728, 738, 744
- **Sender code**: `let _ = pdx_call_checked(SLOT_SHELL, OP_USB_MOUSE_REPORT, ...);`
- **After call**: Continues to next proof stage or event loop
- **No listen/reply wait**: `let _ =` discards the result. No caller-side reply listener.
- **Verdict**: Dead. The reply was never consumed.

### Callsite #3: OP_SCENE_SETTINGS_CMD
- **Sender**: **None** — opcode 0xFB is defined only in silk-shell itself (line 25)
- **No server in the codebase sends this opcode**
- handler does real work (`handle_scene_settings_cmd` persists appearance to sexstore)
- But nobody triggers it
- **Verdict**: Dead. The reply was never sent or consumed.

---

## Patch Applied

**File**: `servers/silk-shell/src/main.rs`

Removed three `pdx_reply(0);` lines — no other changes.

| Line (old) | Removed |
|-----------|---------|
| `9780: pdx_reply(0);` — inside `OP_SHELL_BIND_BUFFER` handler | ✅ |
| `9885: pdx_reply(0);` — inside `OP_USB_MOUSE_REPORT` handler | ✅ |
| `10966: pdx_reply(0);` — inside `OP_SCENE_SETTINGS_CMD` handler | ✅ |

**Safety proof**:
- All three used syscall 1 (unhandled → `_ => u64::MAX`) — **no reply was ever delivered**
- All three used `target_pd=0` (invalid PD) — **no recipient would be found**
- No caller waits for a reply — **nothing blocks on missing reply**
- `mutated = true` is set independently and not affected
- Removing the lines produces **exactly the same runtime behavior** as before

**Not removed**:
- `use` import of `pdx_reply` at line 8 — harmless, will be cleaned up when the shared helper is fixed

---

## Future Fix Plan (for when sex_pdx::pdx_reply is fixed)

After `sex_pdx::pdx_reply` is corrected to `fn pdx_reply(target_pd: u64, val: u64)` with syscall 29:

1. Remove `pdx_reply` from the `use` import at line 8 (no longer needed)
2. If any future opcode handler genuinely needs to send a reply, add:
   ```rust
   pdx_reply(msg.caller_pd, reply_value);
   ```
   Only for opcodes whose callers wait for a reply via `pdx_listen_raw(0)`.
3. Currently no such opcode exists in silk-shell.

---

## Build Result

`./scripts/entrypoint_build.sh` — **passes**.

No runtime behavior change (verified by code analysis: all three callsites were no-ops on broken syscall 1).
