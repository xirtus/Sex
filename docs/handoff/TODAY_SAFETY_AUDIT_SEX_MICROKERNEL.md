# Today Safety Audit — SexOS Microkernel

**Date:** 2026-05-05
**Auditor:** Real Claude (claude-sonnet-4-6)
**Mode:** Report only. No code changed.

---

## A. Commit Range Audited

**Committed (HEAD~30..HEAD):**
J1–J7 (Linen/Quil/Collar/Mesh/Bell shell-local stubs), K2A (buffer_id collision fix),
K2B (namespace design doc), K2C (seed coherence init), Quil PD boot loop, I1/I2/I3
(placeholder surfaces), D1–D5 (accessibility), F1/F2/G1/H1/H2 (spec docs).

**Uncommitted (`M servers/sexstore/src/main.rs`):**
E4–E7 sexstore changes: capability policy gate, value envelope validation,
`REPLY_STATUS_BIT` discriminator, `OP_KV_DEL`/tombstone/generation, structured proof markers.

**Untracked docs:** `docs/handoff/E1–E8_*.md`, `K2B_NAMESPACE_SPEC_DOC_V1.md`

---

## B. Files Inspected

| File | Status | Method |
|------|--------|--------|
| `servers/sexstore/src/main.rs` | Modified (M), uncommitted | `git diff` full review |
| `servers/silk-shell/src/main.rs` | Committed | rg + targeted reads |
| `servers/quil/src/main.rs` | Committed | full read |
| `kernel/src/init.rs` | Committed | rg for grants |
| `crates/sex-pdx/src/lib.rs` | Committed | rg for slots/opcodes |
| `docs/handoff/E1–E5_*.md` | Untracked | head reads |

---

## C. PASS/FAIL Table by Audit Area

| Area | Verdict | Notes |
|------|---------|-------|
| Runtime safety — committed | PASS | No unwrap/panic/invalid indexing in J1-J7/K2. Lifecycle-safe helpers used. |
| Runtime safety — uncommitted E* | FIX_FIRST | CRITICAL bit-63 collision; HIGH missing shell client |
| Namespace/ABI | PASS | 0xB0-0xB2 local-only, no sex-pdx promotion. No opcode collision. |
| Capability topology | PASS | Quil has no display/storage/shell caps. Shell gets SLOT_QUIL outbound only. |
| Renderer ownership | PASS | sexdisplay untouched in all commits. No framebuffer writes outside sexdisplay. |
| Accessibility | PASS with LOW note | D2/D3/D4 correct. One Esc double-dispatch edge case. |
| Storage — committed | N/A | No storage calls in committed silk-shell. |
| Storage — uncommitted | FIX_FIRST | CRITICAL bit-63; HIGH incomplete client. |
| Docs/code consistency | PASS | E* docs describe uncommitted code accurately. |

---

## D. Critical Findings

### CRITICAL-1: `REPLY_STATUS_BIT` bit-63 collision with valid stored values

**Location:** `servers/sexstore/src/main.rs` (uncommitted), GET path + `store_validate_value()`

**Root cause:**
```rust
const REPLY_STATUS_BIT: u64 = 0x8000_0000_0000_0000;
// GET active hit returns raw stored val:
kv_reply(caller, result);
// Caller checks: if reply & REPLY_STATUS_BIT != 0 { /* status, not value */ }
```

For key `0x01`, `store_validate_value()` accepts `magic=0xAC, version=0x01, XOR checksum`.
In little-endian, byte 7 of the u64 = bits 56-63. Checksum = `b[0]^b[1]^...^b[6]`.

```
b[0]=0xAC, b[1]=0x01 → partial XOR = 0xAD
If remaining bytes produce final XOR >= 0x80, byte 7 >= 0x80 → bit 63 SET in stored u64.
```

`store_validate_value()` does NOT enforce `val & REPLY_STATUS_BIT == 0`. A fully valid
envelope value can have bit 63 set. GET returns it raw; caller sees bit 63 = 1 and
misinterprets it as a status reply (`KV_NOT_FOUND`, `KV_FULL`, etc.), NOT the actual data.

**Scope:** Currently latent — no shell caller exists yet for the new protocol (see CRITICAL-2).
But the discriminator scheme is architecturally broken and must be fixed before the caller lands.

**Fix direction:** Choose one:
- A (minimal): In `store_validate_value()` for key 0x01, add `if val & REPLY_STATUS_BIT != 0 { return false; }`. Forces high-byte checksum values >= 0x80 to be rejected at PUT time. Callers must pre-clear bit 63 before storing.
- B (correct): In GET active-hit path, assert `result & REPLY_STATUS_BIT == 0` before `kv_reply(caller, result)`. If assertion fails, reply with KV_INVALID_VALUE. Documents the invariant at the call site.
- C (architectural, larger): Use a separate reply register or side channel instead of bit 63.

Recommendation: **Option A** — enforce at PUT validation, reject bad envelopes at write time.

---

## E. High Findings

### HIGH-1: No shell-side caller for new E* storage protocol

**Location:** `servers/silk-shell/src/main.rs` — zero matches for
`OP_KV_GET|OP_KV_PUT|OP_KV_DEL|0xB0|0xB1|0xB2|REPLY_STATUS_BIT|KV_DENIED|KV_NOT_FOUND`.

The E4–E7 sexstore changes are server-side only. No shell client code implements the new
`REPLY_STATUS_BIT` protocol. The new status codes are untestable end-to-end. The old
pre-E4 protocol used `0` as not-found sentinel (no bit-63 discriminator); there may be
a pre-existing shell caller using the old protocol that would now misinterpret all status
replies as value `0x8000_0000_0000_0000X`.

**Verdict:** E* sexstore changes must NOT be committed standalone. Server and shell-side
client must land together. Either:
- Commit both in one atomic commit, OR
- Keep sexstore uncommitted until the shell caller is written

---

## F. Medium Findings

### MEDIUM-1: `was_update` logs `ok=0` for successful new inserts

**Location:** `servers/sexstore/src/main.rs`, PUT handler, LOG_PUT block.

```rust
serial_println!("[sexstore.kv.put] key={} ok={}", key, if was_update { 1 } else { 0 });
```

`was_update` = true only for key-exists update path, false for new insert. Successful
new inserts log `ok=0` even though PUT succeeded. Misleading in diagnostics but not a
correctness bug. Easy fix: rename `was_update` to `was_key_found_before` or change log
to `inserted={}`.

### MEDIUM-2: Unknown opcode replies `kv_reply(caller, 0)` without `REPLY_STATUS_BIT`

**Location:** `servers/sexstore/src/main.rs`, `_ =>` match arm.

Under new protocol, 0 is a valid stored value (GET success with stored 0). A future
caller sending an unknown opcode gets back 0, which it may interpret as "GET returned 0"
instead of "opcode not handled". Low practical risk (callers know their opcode), but the
unknown-opcode reply should use `kv_reply_status(caller, KV_OK)` or a dedicated error code
once the protocol is finalized.

### MEDIUM-3: Esc double-dispatch when scene settings panel is open

**Location:** `servers/silk-shell/src/main.rs`, HID event loop ~line 7917 and 7952.

When `SCENE_SETTINGS_ACTIVE && !ATLAS_MODE_ENABLED && scancode == 0x01`:
1. Panel intercept fires: closes panel, sets `mutated = true`
2. Code falls through to `scancode_to_action(0x01)` → `AccessZoomToggle`
3. AccessZoomToggle fires: attempts `toggle_zoom_frame(FOCUSED_SURFACE_ID)`

Both actions fire in a single Esc keypress. Zoom toggle on a focused frame fires silently
after panel close. Not a safety issue but unexpected UX behavior. Fix: add `continue` in
the panel intercept match arm for consumed scancodes (0x01, 0x02, 0x03, 0x06) or flag to
skip normal dispatch after panel intercept handles the key.

---

## G. Low Findings / Docs-Only Corrections

### LOW-1: OP_KV_DEL = 0xB2 local-only, not promoted to sex-pdx

Intentional and documented correctly in code comments. No ABI violation. Correct.

### LOW-2: KV_DENIED renumbered 0x01 → 0x05 from E4

Boot log `[sexstore.status.mapping]` emits current mapping. No committed caller
hardcodes old value. Safe. E5 doc documents the old E4 mapping — accurate.

### LOW-3: `was_tombstoned` in PUT revive path logs `old_gen` BEFORE bump

```rust
serial_println!("[sexstore.tombstone.revive] key={} old_gen={}", key, (*slot).generation);
// then:
bump_generation(slot);
serial_println!("[sexstore.generation.bump] key={} ... gen={}", key, idx, (*slot).generation);
```

`old_gen` is the pre-revive generation (correct). The subsequent `generation.bump` logs
the post-bump generation. Log sequence is coherent but subtle. Acceptable.

### LOW-4: D4 `access_label_token` — DJB2 hash over shell-owned static labels only

Code comment correctly states "Only shell-owned static/bounded labels are hashed — never
app-provided names." Hash tokens are logged as `{:#x}` opaque numerics. No claim of
hash irreversibility anywhere in the code. No secrecy boundary crossed. PASS.

---

## H. Namespace / Opcode / Slot Table

| Namespace | Range | Owner | Status |
|-----------|-------|-------|--------|
| PDX slots | 1-11 | IPCPKU_MAP canonical | PASS — no new slots |
| sexdisplay opcodes | 0xEC/0xEE/0xEF | sex-pdx/inline | PASS — unchanged |
| SilkBar opcodes | 0xF0–0xF4 | sex-pdx | PASS — unchanged |
| Quil ping opcode | 0xD0 (OP_QUIL_PING) | sex-pdx | PASS — no collision |
| sexstore opcodes | 0xB0–0xB2 | LOCAL to sexstore+shell | PASS — not in sex-pdx, no collision |
| Linen object IDs | 1-16 | shell-local | PASS — K2A/K2B resolved |
| Quil seed buffer IDs | 1-6 | shell-local | PASS — separate type namespace |
| Quil dynamic buffer IDs | 1001-1016 | shell-local | PASS — K2A base=1000 |
| Surface IDs — app | 100-103 | shell/display shared | PASS — unchanged |
| Surface IDs — workstation | 200-204 | shell/display shared | PASS — unchanged |
| Surface IDs — OS panels | 0x90-0x97 | shell/display shared | PASS — unchanged |

---

## I. Capability Grant Table

| PD | Granted Slots | Via | Status |
|----|--------------|-----|--------|
| silk-shell | SLOT_DISPLAY, SLOT_SHELL, SLOT_SILKBAR, SLOT_SEXSTORE (cond.), SLOT_INPUT, SLOT_QUIL | kernel/src/init.rs | PASS |
| sexdisplay | *(none from shell)* | kernel | PASS |
| quil | *(no SLOT_DISPLAY, no SLOT_SEXSTORE, no SLOT_SHELL)* | kernel | PASS — isolation correct |
| sexinput | SLOT_SHELL (reverse: input→shell) | kernel | PASS |
| sexstore | *(no caps granted to others; only receives)* | kernel | PASS |

No reverse Quil→Shell grant. No Quil→Display grant. No app direct storage cap. All PASS.

---

## J. Storage Status / Reply Table

| Code | Constant | Reply encoding | Shell must check | Status |
|------|----------|---------------|-----------------|--------|
| 0x00 | KV_OK | `REPLY_STATUS_BIT | 0x00` | bit 63 == 1 | PASS |
| 0x01 | KV_NOT_FOUND | `REPLY_STATUS_BIT | 0x01` | bit 63 == 1 | PASS |
| 0x02 | KV_FULL | `REPLY_STATUS_BIT | 0x02` | bit 63 == 1 | PASS |
| 0x03 | KV_INVALID_KEY | `REPLY_STATUS_BIT | 0x03` | bit 63 == 1 | PASS |
| 0x04 | KV_INVALID_VALUE | `REPLY_STATUS_BIT | 0x04` | bit 63 == 1 | PASS |
| 0x05 | KV_DENIED | `REPLY_STATUS_BIT | 0x05` | bit 63 == 1 | PASS |
| raw val | (GET success) | raw `(*slot).val` | bit 63 == 0 | **CRITICAL-1: not enforced** |

---

## K. Accessibility Binding Table

| Scancode | Key | SurfaceAction | Handler | Lifecycle safe? | Status |
|----------|-----|---------------|---------|----------------|--------|
| 0x01 | Esc | AccessZoomToggle | `toggle_zoom_frame()` | Yes — checks alive/tombstone | PASS (MEDIUM-3 double-fire) |
| 0x0F | Tab | AccessFocusNext | `access_handle_keyboard_action()` | Yes — semantic node tree | PASS |
| 0x0E | Backspace | AccessFocusPrev | `access_handle_keyboard_action()` | Yes — semantic node tree | PASS |
| 0x1C | Enter | AccessActivate | `access_handle_keyboard_action()` | Yes — minimize/restore only | PASS |
| 0x57 | F11 | AccessClose | `close_surface_from_frame_light()` | Yes — existing lifecycle helper | PASS |
| — | — | AccessSceneNext/Prev | deferred (no binding yet) | N/A | PASS |

Atlas key intercept (0x44=F10): When `ATLAS_MODE_ENABLED`, all scancodes except F10
go to `handle_atlas_keyboard()`. AccessFocusNext/Prev/Activate/Close/Zoom all safely
consumed there. No leak into editor/app input. PASS.

D2 semantic nodes: skip dead/tombstoned surfaces in tree construction (checked via
`surface_is_alive()` + `is_tombstoned()`). Label tokens are DJB2 hashes of shell-owned
static strings only. No app content logged. PASS.

---

## L. Recommended Fix Prompts

### Fix 1 — CRITICAL-1: Enforce bit-63-free invariant for stored values (sexstore, small)

```
In servers/sexstore/src/main.rs, in store_validate_value():
After all existing key-specific checks, add a final check for ALL accepted values:
  if value & 0x8000_0000_0000_0000u64 != 0 { return false; }
This enforces that no stored value has bit 63 set, preserving the REPLY_STATUS_BIT
discriminator protocol. Add proof marker: [sexstore.value.invalid] reason=bit63_set
Add constant: const REPLY_STATUS_BIT_MASK: u64 = 0x8000_0000_0000_0000;
Use it in the check.
Do not change any other logic. Build must pass.
Commit: fix(sexstore): enforce bit-63-free invariant for stored values
Docs: update docs/handoff/E4_STORAGE_SCHEMA_VALIDATION_V1.md to document this constraint.
```

**Safe for deepseekclaude:** Yes — single constant + one line in validate function.

### Fix 2 — MEDIUM-3: Esc double-dispatch in scene settings panel (silk-shell, small)

```
In servers/silk-shell/src/main.rs, in the SCENE_SETTINGS_ACTIVE match block:
For scancodes that are fully handled by the panel intercept (0x01, 0x02, 0x03, 0x06),
add a skip flag after the match to prevent fallthrough to scancode_to_action().
Pattern: set `let panel_consumed = true;` for consumed scancodes, then wrap the
Atlas/action dispatch in `if !panel_consumed { ... }`.
Do not change AccessZoomToggle handler itself.
Build must pass. Commit: fix(shell): prevent Esc double-dispatch when settings panel active
```

**Safe for deepseekclaude:** Yes — additive bool flag, no handler changes.

### Fix 3 — HIGH-1: Write shell-side caller for E* storage protocol (silk-shell, larger)

**Requires real Claude.** Shell storage protocol uses `REPLY_STATUS_BIT` discriminator
and new status codes. Must be written together with Fix 1 to be testable. See
`docs/handoff/E2_STORAGE_PROTOCOL_SPEC_V1.md` for protocol spec.

### Fix 4 — MEDIUM-1: Fix `was_update` log accuracy (sexstore, trivial)

Change final `LOG_PUT` serial_println to log actual PUT outcome:
```
serial_println!("[sexstore.kv.put] key={} result={}", key, if full { "full" } else { "ok" });
```
**Safe for deepseekclaude:** Yes — log-only change.

---

## M. Final Verdict

```
SAFE WITH FIXES FIRST
```

| Scope | Verdict | Blocker |
|-------|---------|---------|
| **Committed J1-J7/K2A-K2C** | **SAFE TO CONTINUE** | None — already clean |
| **Uncommitted E4-E7 sexstore** | **FIX_FIRST** | CRITICAL-1 (bit-63 collision) + HIGH-1 (missing shell client) |
| **D1-D5 accessibility (committed)** | **SAFE TO CONTINUE** | MEDIUM-3 is UX-only, not safety-blocking |

**Critical/High/Medium/Low finding counts:**
- CRITICAL: 1 (bit-63 REPLY_STATUS_BIT collision)
- HIGH: 1 (no shell-side protocol client)
- MEDIUM: 3 (was_update log, unknown-op reply, Esc double-dispatch)
- LOW: 4 (OP_KV_DEL local-only, KV_DENIED renumber, tombstone log order, label_token)

**Namespace collisions:** NONE in committed work
**Capability drift:** NONE — topology clean
**Runtime risk:** ZERO in committed code; LATENT in uncommitted E* (not reachable until shell caller written)
**Build:** `[SEXOS ENTRYPOINT] success` — ISO clean

**Recommended next prompt:** Fix 1 (bit-63 invariant enforcement) — single-line add in
`store_validate_value()`, safe for deepseekclaude, unblocks E* commit path.
