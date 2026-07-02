# QUIL_PALETTE_ACTION_STUBS_V1

**Status:** Implemented
**Date:** 2026-05-06
**Purpose:** Make Quil palette Enter actions meaningful as internal marker-only
command stubs. No command execution, no ABI changes.

---

## 1. Command Table

### Command IDs

| ID | Constant | Purpose | Executes? |
|----|----------|---------|-----------|
| 1 | `CMD_NEW_BUFFER_STUB` | Create a new Quil buffer slot | No — marker only |
| 2 | `CMD_OPEN_OBJECT_STUB` | Open a Linen object in Quil | No — marker only |
| 3 | `CMD_AGENT_REVIEW_STUB` | Review agent task output | No — marker only |
| 4 | `CMD_RUN_CHECK_STUB` | Run a diagnostic/consistency check | No — marker only |
| 5 | `CMD_SETTINGS_STUB` | Open Quil settings panel | No — marker only |

### Row-to-command mapping (PALETTE_COMMANDS)

```
row 0 → CMD_NEW_BUFFER_STUB   (1)
row 1 → CMD_OPEN_OBJECT_STUB  (2)
row 2 → CMD_AGENT_REVIEW_STUB (3)
row 3 → CMD_RUN_CHECK_STUB    (4)
row 4 → CMD_SETTINGS_STUB     (5)
```

Defined as a const fixed-size array with compile-time row count matching
`QUIL_ROWS = 5`. Lookup via `palette_command_for_row(row)` — returns 0
for out-of-range.

---

## 2. Enter Behavior

On Enter key (action == 3, scancode `0x1c` / 28):

```
1. if palette_active:
   a. cmd = PALETTE_COMMANDS[selected_row]
   b. emit marker: [quil.palette.action] row=<n> cmd=<id>
   c. stub complete — no further action
2. if !palette_active:
   a. emit: [quil.palette.reject] action=enter reason=inactive
```

No visual pulse added. The palette remains visible after Enter (no dismiss).
No command executes. No shell-call, spawn, storage, or ABI interaction.

---

## 3. Marker Behavior

### Palette markers emitted on Enter

```
[quil.palette.action] row=0 cmd=1
```

Each marker contains:
- `row=N` — the palette row index (0-4) that was selected when Enter fired
- `cmd=N` — the numeric command ID from the lookup table

### Pre-existing markers unchanged

| Marker | Trigger |
|--------|---------|
| `[quil.palette.key] scancode=0x1c action=3` | Raw key decode (budgeted) |
| `[quil.palette.draw] rows=5 selected=N` | Palette redraw (budgeted) |
| `[quil.palette.selected] row=N` | Selected row accent bar (budgeted) |
| `[quil.palette.row] row=N selected=0/1` | Each row draw (budgeted) |
| `[quil.palette.reject] action=up/down/enter/esc/key reason=...` | Rejected input |

---

## 4. Code Changes

### Changed file: `servers/quil/src/main.rs`

| Lines | Change |
|-------|--------|
| 31-51 | Added command ID constants + lookup table + helper function |
| 244-251 | Modified Enter handler: replace `kind=enter` with `row=<n> cmd=<id>` |

### Diff summary

```
 servers/quil/src/main.rs | 27 +++++++++++++++++++++++++--
 1 file changed, 25 insertions(+), 2 deletions(-)
```

---

## 5. Build Result

**PASS.** `./scripts/entrypoint_build.sh` succeeds. Probe confirms:

```
[quil.palette.key] scancode=0x1c action=3
[quil.palette.action] row=0 cmd=1
```

No new warnings. No ABI changes.

---

## 6. Future Blocked Work

### What remains blocked

| Feature | Blocked by | Required change |
|---------|-----------|-----------------|
| Real `NewBuffer` execution | Shell/Quil ABI for buffer creation | New opcode or OP_QUIL_PING extension |
| Real `OpenObject` execution | Shell/Quil ABI for object→buffer link | Shell must accept a "open in Quil" request |
| Real `AgentReview` execution | Shell/Quil ABI for agent task dispatch | Agent PD capability + message format |
| Real `RunCheck` execution | Diagnostic check infrastructure | No diagnostic runtime exists yet |
| Real `Settings` execution | Quil-internal settings state machine | Settings protocol + storage path |
| Visual pulse on Enter | Desired but deferred | Requires redraw loop without blocking listen |

### ABI boundary (do not cross without STOP FIRST)

Any command that requires:
- Creating a new PDX buffer slot on the shell
- Spawning or signaling another domain
- Reading/writing to sexstore
- Modifying shell surface lifecycle

...requires a new shell→Quil opcode or a Quil→shell request mechanism.
Neither exists in V1. The stubs document intent for when that ABI boundary
opens.
