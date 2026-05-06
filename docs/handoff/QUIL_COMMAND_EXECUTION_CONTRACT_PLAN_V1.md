# QUIL_COMMAND_EXECUTION_CONTRACT_PLAN_V1

**Status:** Design — No Implementation
**Date:** 2026-05-06
**Purpose:** Design the future safe contract for turning Quil palette command stubs
into real actions. Docs only. ABI remains frozen.

---

## 1. Current V1 State

### Implemented (QUIL_PALETTE_ACTION_STUBS_V1)

Quil has a 5-row palette navigable by Up/Down. Enter emits a marker:

```
[quil.palette.action] row=0 cmd=1
```

| Aspect | V1 behavior |
|--------|-------------|
| Command IDs | Fixed constants 1..5 in a `PALETTE_COMMANDS` const array |
| Enter handler | Resolves row→cmd, emits marker, does nothing else |
| Execution | **None.** No shell call, no spawn, no storage, no lifecycle change |
| Palette dismiss | Only on Esc (`action=4`) — palette stays visible on Enter |
| ABI | Frozen. No new opcodes, no new capability grants |

### Quil's current capabilities

| Slot | Destination | Used for |
|------|-------------|----------|
| `SLOT_DISPLAY` (5) | sexdisplay | 0xEF fill rects |
| `SLOT_QUIL` (11) | self | `pdx_listen_raw(0)` — receives OP_QUIL_PING, OP_HID_EVENT |

**Quil does NOT have `SLOT_SHELL` (6).** It cannot send any message to the
shell. All current communication is shell→Quil (OP_QUIL_PING, OP_HID_EVENT).

---

## 2. Ownership Model (Future)

### Definitive boundaries

| Domain | Owns | Does Not Own |
|--------|------|--------------|
| **Quil** (app server) | Palette state, selected row, command intent, local panel drawing | Shell lifecycle, buffer creation, object links, app spawn, screen layout |
| **silk-shell** (authority) | Lifecycle, buffer table, object→buffer links, surface geometry, focus | App-local draw state, palette selection, per-command authorization logic |
| **Linen** (object model) | Object table, object metadata, grant_refs | Command routing, app execution, screen layout |
| **Collar** (authority, future) | Capability grants, operation approval, identity verification | Command policy, lifecycle, surface geometry |
| **sexdisplay** (compositor) | Framebuffer, surface registry, clipping | Layout policy, command routing, authorization |

### Command flow (future, all layers)

```
Quil ──[command request]──→ Shell ──[dispatch]──→ Linen / Collar / Agent PD / self
                                    │
                                    └──[reject]──→ Quil (invalid, denied, busy)
```

- **Quil** sends a command request to the shell (direction: Quil→Shell).
- **Shell** validates the command, checks surface lifecycle, and either:
  - Executes internally (NewBufferStub → buffer table insert)
  - Forwards to another domain (OpenObjectStub → Linen link)
  - Rejects with reason code
- **Collar** (future) approves or denies the operation before execution.
- **sexdisplay** never sees commands.

---

## 3. Proposed Future Request Shape

### Quil → Shell command request

When ABI changes are permitted, the smallest addition is:

```
A. Grant quil capability: SLOT_SHELL (6) in kernel/src/init.rs
B. Define new opcode: OP_QUIL_COMMAND (opcode TBD in an unfrozen ABI phase)
C. Quil calls: pdx_call(SLOT_SHELL, OP_QUIL_COMMAND, source_surface_id, cmd_id, row_info)
D. Shell receives in pdx_listen_raw(0), matches OP_QUIL_COMMAND, dispatches
```

### Request payload

```
arg0 = source_surface_id  (e.g. SURFACE_ID_QUIL = 201)
arg1 = command_id         (1..5, matching PALETTE_COMMANDS)
arg2 = sequence_token     (monotonic counter; enables shell to deduplicate or
                           reject stale commands after palette re-open)
```

No strings. No text content. No buffer handles. No grant_refs in the
request itself (grants are checked by Collar at dispatch time, not
embedded in the command).

### Shell response

The shell **does not reply** to Quil. The command is fire-and-forget:

- Success: shell acts silently (or emits own marker)
- Rejection: shell emits `[shell.quil.cmd.reject]` marker (visible in log)

This avoids blocking Quil's listen loop and keeps the ABI surface minimal.

---

## 4. Command Policy (Per-Command Ownership)

### 4.1 CMD_NEW_BUFFER_STUB (cmd=1)

| Aspect | Future behavior |
|--------|-----------------|
| Owner | Shell (silk-shell) — buffer table is shell-local |
| Action | Insert a new `QuilBuffer` slot in `QUIL_BUFFERS`; re-render buffer list |
| Requires | Shell-internal buffer insert function already exists (`quil_buffer_table_init` pattern but for dynamic insert) |
| Rejection | Buffer table full (max 16); allocate new dynamic buffer ID |
| ABI needed | Yes — Quil→Shell request opcode |

### 4.2 CMD_OPEN_OBJECT_STUB (cmd=2)

| Aspect | Future behavior |
|--------|-----------------|
| Owner | Shell → Linen (linked object table) |
| Action | Shell calls `open_linen_object_in_quil(selected_object_id)` — already exists |
| Requires | Shell must know which Linen object is currently selected. Either: (a) Quil sends object_id in command, or (b) shell uses `SELECTED_LINEN_OBJECT_ID` |
| Rejection | No object selected; Collar deny; Linen object dead |
| ABI needed | Yes — plus object_id may need to flow from Linen→Quil or Shell→Quil |

### 4.3 CMD_AGENT_REVIEW_STUB (cmd=3)

| Aspect | Future behavior |
|--------|-----------------|
| Owner | Agent PD (not yet spawned) |
| Action | Route command to agent PD; agent returns review output as a new buffer |
| Requires | Agent PD spawn + capability grant + agent→Quil message format |
| Rejection | No agent PD; agent busy; agent has no output for this object |
| ABI needed | Yes — plus new agent→Quil opcode, plus agent PD spawn |

### 4.4 CMD_RUN_CHECK_STUB (cmd=4)

| Aspect | Future behavior |
|--------|-----------------|
| Owner | Diagnostic runtime (not yet built) |
| Action | Run a consistency check on current project/buffer state; emit result marker |
| Requires | Diagnostic check infrastructure — no runtime exists yet |
| Rejection | No diagnostic runtime; check type unknown |
| ABI needed | Yes — plus diagnostic PD or shell-internal check runner |

### 4.5 CMD_SETTINGS_STUB (cmd=5)

| Aspect | Future behavior |
|--------|-----------------|
| Owner | Shell (scene settings protocol) or Quil-internal settings state machine |
| Action | Open settings panel or toggle Quil-local preferences |
| Requires | Settings protocol path (already exists for scene presets in shell) or Quil-internal settings state |
| Rejection | Already in settings; no settings infrastructure |
| ABI needed | Yes — unless handled entirely inside Quil (Quil-local toggle, no shell call) |

---

## 5. Rejection Policy

### Rejection cases (shell-side, future)

| Case | Condition | Marker |
|------|-----------|--------|
| Invalid command ID | `cmd_id` not in 1..5 | `[shell.quil.cmd.reject] reason=invalid_cmd id=N` |
| Stale Quil surface | Quil surface is tombstoned, hidden, or inactive | `[shell.quil.cmd.reject] reason=stale_surface sid=N` |
| Missing capability | Collar denies operation (future) | `[shell.quil.cmd.reject] reason=collar_deny op=N` |
| Shell busy | Shell cannot dispatch (e.g. in drag) | `[shell.quil.cmd.reject] reason=busy` |
| Deprecated request | `sequence_token` out of order (replay) | `[shell.quil.cmd.reject] reason=stale_token` |

### Rejection cases (Quil-side, current + future)

| Case | Condition | Current? |
|------|-----------|----------|
| Palette inactive | `palette_active == false` | Yes — `[quil.palette.reject] action=enter reason=inactive` |
| Unmapped key | Scancode not in palette decode map | Yes — `[quil.palette.reject] action=key reason=unmapped` |
| Row overflow | `rows_bottom > PANEL_Y + PANEL_H` | Yes — `[quil.palette.reject] action=draw reason=row_overflow` |
| No SLOT_SHELL | Quil cannot send command | **Current V1** — no cap granted, no rejection possible |

---

## 6. ABI Impact

### Changes required for any command execution

| Change | Type | Scope |
|--------|------|-------|
| Grant `SLOT_SHELL` to quil in `kernel/src/init.rs` | Capability grant | Kernel (STOP FIRST) |
| Define `OP_QUIL_COMMAND` opcode | ABI addition | `crates/sex-pdx/src/lib.rs` (STOP FIRST) |
| Quil calls `pdx_call(SLOT_SHELL, OP_QUIL_COMMAND, ...)` | Quil code | `servers/quil/src/main.rs` |
| Shell matches `OP_QUIL_COMMAND` in main loop | Shell code | `servers/silk-shell/src/main.rs` |
| Rejection markers in shell | Documentation | `servers/silk-shell/src/main.rs` |

**All blocked until ABI phase opens.** Do not implement any of the above
without a formal ABI-unfreeze decision.

### What does NOT change

- No new slots or domains
- No sexdisplay opcode changes
- No sex-pdx ABI removal or rename
- No new text/strings in protocol
- No shared memory or backing buffers
- No framebuffer access for Quil

---

## 7. Smallest Future Implementation Prompt (BLOCKED)

```
WHEN ABI PHASE OPENS:

1. kernel/src/init.rs:
   - Grant SLOT_SHELL to quil:
     pd.grant_capability(sex_pdx::SLOT_SHELL, CapabilityData::Domain(silkshell_id));

2. crates/sex-pdx/src/lib.rs:
   - pub const OP_QUIL_COMMAND: u64 = 0x??;  // pick unused opcode

3. servers/quil/src/main.rs:
   - In Enter handler, after emitting marker:
       pdx_call(SLOT_SHELL, OP_QUIL_COMMAND, SURFACE_ID_QUIL, cmd as u64, 0);
   - No reply expected. No listen-loop change.

4. servers/silk-shell/src/main.rs:
   - In main pdx_listen_raw match, add arm for OP_QUIL_COMMAND:
       Dispatch by command_id to:
       - 1: buffer table insert stub
       - 2: open_linen_object_in_quil (if object selected)
       - 3,4,5: emit [shell.quil.cmd.unimplemented] cmd=N
   - Reject cases checked before dispatch.

5. Budgeted markers in shell:
   - [shell.quil.cmd.recv] cmd=N row=R
   - [shell.quil.cmd.reject] reason=... cmd=N
   - [shell.quil.cmd.dispatch] cmd=N action=...
```

---

## 8. Summary

| Question | Answer |
|----------|--------|
| V1 state | Marker-only stubs. No ABI change. No command execution. |
| Future request shape | Quil→Shell `pdx_call(SLOT_SHELL, OP_QUIL_COMMAND, sid, cmd, seq)` — fire-and-forget, no reply. |
| Ownership model | Quil owns intent; Shell owns dispatch; Linen/Collar/Agent own downstream actions; sexdisplay renders only. |
| Rejection policy | 5 shell-side cases, 3 Quil-side cases. All documented in §5. |
| ABI impact | **Blocked.** Requires kernel cap grant + new opcode + shell dispatch. |
| Next implementation | **None in this phase.** Prompt in §7 is frozen until ABI phase opens. |
