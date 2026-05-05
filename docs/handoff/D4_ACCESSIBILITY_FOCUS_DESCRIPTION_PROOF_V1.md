# D4_ACCESSIBILITY_FOCUS_DESCRIPTION_PROOF_V1

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Adds compact focus-description proof logging using the D2 semantic node tree.
No narrator, no speech, no audio, no UI rendering. Proof-log only.

When shell focus changes through any path (keyboard, click, panel, Quil),
emits structured numeric tokens describing the focused semantic node:
role ID, state flags, action flags, target surface/frame/scene IDs, and
a deterministic label hash.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +65 lines (3 helper functions + 1 call in `try_set_focus()`) |
| `docs/handoff/D4_ACCESSIBILITY_FOCUS_DESCRIPTION_PROOF_V1.md` | New handoff doc |

---

## Description Model

### Single Hook

One call to `access_describe_focus()` inserted at the end of `try_set_focus()`,
after the existing D2 `[access.focus.describe]` human-readable marker. This
covers ALL focus change paths:

| Path | Calls `try_set_focus()`? | Covered by D4? |
|------|-------------------------|----------------|
| D3 Tab/Backspace focus traversal | ✅ Yes | ✅ |
| D3B Close/Zoom (operates on focused surface) | ✅ Via prior focus set | ✅ |
| Click-to-focus | ✅ Yes | ✅ |
| Panel toggle focus | ✅ Yes | ✅ |
| Quil open/restore focus | ✅ Yes | ✅ |
| SilkBar workspace switch | ✅ Yes | ✅ |
| Atlas mode exit focus return | ✅ Yes | ✅ |

### Helper Functions

#### `access_label_token(label: &[u8; 32]) -> u32`
Simple DJB2-like hash over null-terminated bytes. No heap, no String.
```
hash = 5381
for &b in label.bytes until null:
    hash = hash * 33 + b
```

#### `unsafe fn access_describe_node(node: &AccessNode)`
Emits structured numeric description:
```
[access.focus.describe] node_id=N role=R state=S actions=A target_sid=SID target_fid=FID target_scene=SC label_token=H
[access.focus.label_token] node_id=N token=H
```

#### `unsafe fn access_describe_focus()`
Orchestrator: builds D2 semantic tree → finds focused node → validates target → calls `access_describe_node()`.

---

## Label Privacy Invariant

> D4 focus description logs only shell-owned role/id/state/action tokens.
> It must not log app text, document names, file names, user content,
> or future Quil buffer contents.

The `access_label_token()` function hashes the D2 `[u8; 32]` label to a
`u32` token. This is a one-way deterministic hash — it cannot be reversed
to recover the original label. Only shell-owned static/bounded labels
from the D2 model are hashed (e.g., "Frame", "Quil", "Linen", "Mesh").
Future app-provided names must never be logged as plaintext — only
through this hash, or omitted entirely.

---

## Dead-Target Filtering

Every focus description validates the target before logging:

1. `access_emit_shell_nodes()` builds the tree (already excludes dead frames)
2. Focused node is found by `FOCUSED_SURFACE_ID` match
3. `surface_is_alive(sid) && !is_tombstoned(sid)` check
4. If dead → `[access.focus.describe.skip_dead]` instead of description

This is a safety net — `try_set_focus()` already validates targets before
setting focus, so dead targets should never reach the description hook.

---

## Proof Markers Added

| Marker | Budget | Location | When |
|--------|--------|----------|------|
| `[access.focus.describe]` | 32 | `access_describe_node()` | Structured numeric description of focused node |
| `[access.focus.label_token]` | 32 | `access_describe_node()` | Numeric label hash for focused node |
| `[access.focus.describe.skip_dead]` | 8 | `access_describe_focus()` | Focus target dead/tombstoned (safety net) |
| `[access.focus.describe.reject]` | 8 | `access_describe_focus()` | Cannot produce description (no focus, empty tree, not found) |

### Example output

```
[access.focus.describe] node_id=8193 role=7 state=0x5 actions=0x2d target_sid=100 target_fid=0 target_scene=0 label_token=0x1885d250
[access.focus.label_token] node_id=8193 token=0x1885d250
```

Where:
- `role=7` = `AccessRole::Frame`
- `state=0x5` = `ACCESS_FOCUSED | ACCESS_VISIBLE`
- `actions=0x2d` = `ACT_FOCUS | ACT_ACTIVATE | ACT_MINIMIZE | ACT_CLOSE | ACT_ZOOM`

---

## Behavior Changes

**None.** All D4 code is pure logging. No focus, lifecycle, frame, or
interaction state is ever mutated. The `access_describe_focus()` call
is the last operation in `try_set_focus()` before `return true`.

---

## STOP FIRST Findings

| Condition | Finding |
|-----------|---------|
| Requires String/heap/broad refactor | ✅ Not needed — label hash uses fold over `[u8; 32]` |
| Requires app-content inspection | ✅ Not needed — all labels from D2 shell model |
| Changes focus behavior | ✅ No — pure logging, never mutates state |
| Requires narrator/speech/audio | ✅ Not added — proof markers only |
| Requires kernel/ABI change | ✅ Not needed |
| Requires sexdisplay/framebuffer change | ✅ Not needed |
| Focus change hooks too scattered | ✅ Single hook in `try_set_focus()` — covers all paths |
| Logs app content/names as strings | ✅ No — only numeric tokens and deterministic hashes |
| Requires persistence/storage | ✅ Not needed |

**No STOP FIRST conditions triggered.**

---

## References

- `docs/handoff/D3B_ACCESSIBILITY_KEYBOARD_ACTIONS_COMPLETE_V1.md` — D3B close/zoom
- `docs/handoff/D3_ACCESSIBILITY_KEYBOARD_ACTIONS_V1.md` — D3 focus traversal
- `docs/handoff/D2_ACCESSIBILITY_SEMANTIC_NODE_EMITTER_V1.md` — D2 node model
- `docs/handoff/D1_ACCESSIBILITY_SHELL_SEMANTICS_AUDIT_V1.md` — D1 audit
- `docs/D_ACCESSIBILITY_STACK_PLAN_V1.md` — Track D plan
- `servers/silk-shell/src/main.rs` — implementation (~65 lines added)
