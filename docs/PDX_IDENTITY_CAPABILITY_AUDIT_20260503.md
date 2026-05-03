# PDX Identity / Capability Proof Plan — Audit Report

**Date:** 2026-05-03
**Read first:** `crates/sex-pdx/src/lib.rs`, `kernel/src/syscalls/mod.rs`, `kernel/src/ipc.rs`,
`kernel/src/ipc/messages.rs`, `kernel/src/core_local.rs`,
`servers/sexdisplay/src/main.rs`, `servers/silk-shell/src/main.rs`

---

## 1. Current PDX Identity Model

### How sender identity is established

The identity chain is:

1. **Kernel-stamped at send time** — In `kernel/src/ipc.rs` line 175-183, `traverse_edge()` constructs an `IpcCall` message containing `caller_pd` derived from `current_pd.id` (line 193: `let caller_pd_id = current_pd.id`).

2. **Not user-supplied** — The `caller_pd` field in `MessageType::IpcCall` is set by the kernel in `traverse_edge()`. The arguments supplied by userspace (`arg0`, `arg1`, `arg2`, `opcode`) are passed through as opaque data alongside the kernel-stamped `caller_pd`.

3. **Delivered to receiver** — When the receiver calls `pdx_listen_raw()` (syscall 28), the kernel returns `(type_id, caller_pd, arg0, arg1, arg2)` in registers. The sex-pdx crate packs these into the `PdxMessage` struct (lines 52-59 of `crates/sex-pdx/src/lib.rs`).

4. **PD identity from CoreLocal** — `current_pd_ref()` reads the current PD pointer from `CoreLocal::current_pd_ptr` (GS-base), which is set by `set_pd()` during scheduler switch.

### Where identity is checked vs. ignored

| Server | Checks `caller_pd`? | What it depends on |
|---|---|---|
| **sexdisplay** | **YES** — verifies `slot.owner_pd != msg.caller_pd` on 0xEC, 0xEB, 0xEE, 0xEF | Owner PD bound at surface create time; rejects foreign ops |
| **silk-shell** | **NO** — dispatches on `msg.type_id` only | Relies on slot/opcode only |
| **sexinput** | **NO** — dispatches on `req.type_id` only | Relies on slot/opcode only |
| **silkbar** | **NO** — dispatches on `msg.type_id` only | Relies on slot/opcode only |
| **sexusb** | **NA** — sender only | Sends to sexinput via capability slot |

### Spoof risk assessment

**Low risk** because `caller_pd` is kernel-stamped. A sender cannot forge caller_pd in its arguments — the kernel overwrites any user-supplied value with the real `current_pd.id`.

**However**, servers that ignore `caller_pd` can be attacked via:
- **Opcode guessing** — if an attacker knows the opcodes another server listens on, it can send fake messages
- **Slot-level access** — sender must have a capability grant to the target slot (capability model provides first-level access control)

The practical risk is **opcode stuffing**: a compromised or malicious PD with a valid capability grant to, say, `SLOT_DISPLAY` could send arbitrary opcodes as long as it gets past the opcode-level dispatch.

**sexdisplay's ownership check mitigates this** for surface operations — a malicious PD with SLOT_DISPLAY access could not manipulate surfaces it doesn't own.

---

## 2. Opcode/Capability Failure Behavior

### What happens on unknown opcode?

Depends on the receiver server:

| Server | Unknown opcode behavior |
|---|---|
| **sexdisplay** | Falls through match arms to `serial_println!("TODO: sexdisplay unknown opcode msg={:#x}", msg.type_id)` — **no crash, logs only** |
| **silk-shell** | Falls through match arms silently — **no log, no crash** (message ignored) |
| **sexinput** | Falls through match arms: `serial_println!("[sexinput] unknown type_id={:#x}", req.type_id)` — **logs, continues** |
| **silkbar** | Falls through match arms — **no log, no crash** |

### What happens on invalid target slot?

Kernel returns `ERR_CAP_INVALID` (0xFFFF_FFFF_FFFF_FFFC) from `safe_pdx_call()` (line 194 of `kernel/src/ipc.rs`).

Servers propagate this: `pdx_call_checked` returns `Err(status)`, but in practice most servers use `pdx_call` and **discard the return status**.

### What happens on queue full?

`enqueue()` returns `Err(())` which `traverse_edge()` maps to `ERR_SERVICE_NOT_READY` (0xFFFF_FFFF_FFFF_FFFE). The sender gets the error but there is **no explicit log marker** indicating a dropped message.

### Return value consistency

`pdx_call()` returns `(status, value)` where:
- `status == 0` means success
- `status == ERR_CAP_INVALID` means slot/capability error
- `status == ERR_SERVICE_NOT_READY` means queue full / service busy

**Known inconsistency**: The kernel's `dispatch()` for syscall 0 stores `status` in `regs.rsi` (line 151: `regs.rsi = value`) and returns `status` as the function result — which becomes the `rax` return. So `pdx_call` properly returns `(status=rax, value=rsi)`.

### Are failures grep-able?

| Failure | Marker | Grep-able? |
|---|---|---|
| Unknown opcode in sexdisplay | `TODO: sexdisplay unknown opcode msg=...` | ✅ Yes |
| Unknown opcode in sexinput | `[sexinput] unknown type_id=...` | ✅ Yes |
| Unknown opcode in silk-shell | **None** | ❌ No |
| Unknown opcode in silkbar | **None** | ❌ No |
| Ownership rejection in sexdisplay | `AUTH: 0xEC upsert rejected ...` | ✅ Yes (rate-limited) |
| ERR_CAP_INVALID | kernel log | ⚠️ Not standardized |
| Queue full | kernel log | ❌ Not explicitly logged |

---

## 3. Missing Proof Markers

The following proof markers are **absent** from all servers:

| Marker | Purpose | Where needed |
|---|---|---|
| `[pdx.opcode.unknown]` | Log on unknown opcode | silk-shell, silkbar |
| `[pdx.identity.skip]` | Log that caller_pd was not checked | all four servers on receive |
| `[pdx.identity.accept]` | Log when caller_pd passes ownership check | sexdisplay (already has `AUTH:` prefix) |
| `[pdx.queue.full]` | Log on ring full | kernel ipc.rs |

---

## 4. Opcode Drift Risk

Each server defines opcodes in its own local scope with `const` values:

- **silk-shell**: `OP_DISPLAY_SET_SNAPSHOT=0x15`, `OP_SHELL_BIND_BUFFER=0x14`, `OP_HID_EVENT=0x202`, `OP_USB_MOUSE_REPORT=0x260`, `OP_SURFACE_UPDATE=0xEB`, `OP_SURFACE_DESTROY=0xEE`, etc.
- **sexdisplay**: expects opcodes `0xEC`/`0xEB`/`0xED`/`0xEE`/`0xEF` (as match arm literals in main.rs) plus `0x11` (OP_PRIMARY_FB), `0xF2` (OP_SILKBAR_UPDATE)
- **sexinput**: uses `OP_USB_MOUSE_REPORT=0x260`, `0x201` (raw input)
- **silkbar**: expects `OP_SILKBAR_WORKSPACE_ACTIVE=0xF3`, `OP_SILKBAR_FOCUS_STATE=0xF4`

**Risk**: Opcodes are duplicated between `crates/sex-pdx/src/lib.rs` and server-local `const` definitions. If one changes without the other, senders and receivers silently drift.

---

## 5. Smallest Safe Next Patch

### Patch A — Add proof markers for unknown opcode paths

**Files:** `servers/silk-shell/src/main.rs`, `servers/silkbar/src/main.rs`

Add a `serial_println!("[pdx.opcode.unknown] type_id={:#x}", msg.type_id)` in the fallthrough match arm of each server's receive loop.

**No kernel edits. No sex-pdx edits. No ABI changes.**

### Patch B — Add proof marker for identity-skip on receive

**Files:** `servers/silk-shell/src/main.rs`, `servers/sexinput/src/main.rs`, `servers/silkbar/src/main.rs`

Each server currently ignores `msg.caller_pd`. Add a single log at the top of the receive loop:
```rust
serial_println!("[pdx.recv] type_id={:#x} caller_pd={}", msg.type_id, msg.caller_pd);
```
This logs identity data for every received message without changing behavior.

### Patch C — Add proof marker for sexdisplay ownership accept

**Files:** `servers/sexdisplay/src/main.rs`

Currently ownership violations are logged (`AUTH: reject`), but successful ownership checks are not. Add a rate-limited `[pdx.identity.accept]` on ownership pass.

### Patch D — Add `[pdx.queue.full]` marker in kernel

**STOP FIRST** — requires kernel edit. **Defer** unless audit determines it's critical.

---

## 6. STOP FIRST Conditions

Do NOT proceed if any patch requires:
- Kernel ABI changes
- sex-pdx crate changes
- Syscall number changes
- Capability model changes
- Scheduler / context-switch changes
- New message types in the kernel enum
- Broad refactoring of PDX receive loops

Patches A-C are **server-only changes** — adding log markers to fallthrough match arms and receive loop heads.

---

## 7. Validation Commands

```bash
# Build
./scripts/entrypoint_build.sh

# Verify proof markers appear
grep -E "pdx.opcode.unknown|pdx.recv|pdx.identity.accept|pdx.identity.skip" \
  servers/silk-shell/src/main.rs \
  servers/silkbar/src/main.rs \
  servers/sexinput/src/main.rs \
  servers/sexdisplay/src/main.rs

# Boot and verify runtime markers
grep -aE "pdx.opcode.unknown|pdx.recv|pdx.identity.accept" /tmp/stable.log

# Verify no kernel/sex-pdx edits
git diff -- kernel/ crates/sex-pdx/ | wc -l
# Expected: 0
```

---

## 8. Can Codex Implement Without Kernel/ABI/sex-pdx Edits?

**Yes** — Patches A, B, C are server-only log marker additions:

| Patch | Files | Edits |
|---|---|---|
| A | `servers/silk-shell/src/main.rs`, `servers/silkbar/src/main.rs` | Add `serial_println` to fallthrough match arm |
| B | `servers/silk-shell/src/main.rs`, `servers/sexinput/src/main.rs`, `servers/silkbar/src/main.rs` | Add `serial_println!("[pdx.recv] ...")` at receive loop head |
| C | `servers/sexdisplay/src/main.rs` | Add `serial_println!("[pdx.identity.accept] ...")` on ownership pass |

**Zero kernel edits. Zero sex-pdx edits. Zero ABI changes. Zero capability model changes.**

Patch D (kernel `[pdx.queue.full]`) requires a STOP FIRST kernel edit and should be deferred.

---

## Summary

| Dimension | Status |
|---|---|
| caller_pd spoofable? | **No** — kernel-stamped from `current_pd.id` |
| caller_pd checked everywhere? | **No** — only sexdisplay checks it |
| Opcode failure logged? | **Partial** — sexdisplay/sexinput log, silk-shell/silkbar silent |
| Hostile PD with slot access can do? | Send arbitrary opcodes, but sexdisplay surface ops gated by ownership |
| Next safe step | Add proof markers (server-only, no kernel/ABI) |
| STOP FIRST needed for kernel? | **Not yet** — server markers are sufficient first step |

*End of PDX identity/capability audit report.*
