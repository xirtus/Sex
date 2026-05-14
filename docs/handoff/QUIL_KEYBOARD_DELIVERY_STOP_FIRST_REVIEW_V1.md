# QUIL_KEYBOARD_DELIVERY_STOP_FIRST_REVIEW_V1

## Status: PASS — Root cause identified, Phase 1 fix safe (no STOP FIRST)
Date: 2026-05-14
Attempts: 1
Implementation: **NONE** — audit and design only

---

## Evidence Summary

Shell sends:
```
[silk-shell.key.route.send] target=quil sid=201 scancode=0x24 slot=11 type=0x202 status=0 err=0  (×3)
[silk-shell.key.route.fail] count=0
```

Quil liveness:
```
[quil.init.start]  present
[quil.ready]       present
```

Quil receive:
```
[quil.pdx.recv]  count=0
[quil.key.recv]  count=0
[quil.pdx.listen] count=0  (main-loop marker, fires only when pdx_listen_raw returns)
```

Faults: 0

---

## Files Inspected

| File | Lines | Purpose |
|------|-------|---------|
| `crates/sex-pdx/src/lib.rs` | 223-266, 470-490 | `pdx_listen_raw`, `pdx_call` implementation |
| `kernel/src/syscalls/mod.rs` | 80-203 | Syscall 0 (send) and 28 (listen) dispatch |
| `kernel/src/ipc.rs` | 148-189 | `resolve_edge`, `traverse_edge`, `safe_pdx_call` |
| `kernel/src/init.rs` | 400-456 | SLOT_QUIL capability grant (shell→quil) |
| `servers/silk-shell/src/main.rs` | 5983-5987, 15679-15682, 8358 | Shell Quil key route and ping |
| `servers/quil/src/main.rs` | 455-490, 741-885, 884-920 | Boot proofs, `pdx_call_and_reply`, main loop |

---

## Exact Call Graph

### Shell Send Path
```
handle_hid_event() (shell)
  FOCUSED_SURFACE_ID == SURFACE_ID_QUIL
  pdx_call(SLOT_QUIL=11, OP_HID_EVENT=0x202, scancode, value, EV_KEY)
    → kernel syscall 0, rdi=11, rsi=0x202
    → safe_pdx_call(cap_id=11, opcode=0x202, ...)
      → find_capability(11) → CapabilityData::Domain(quil_pd_id)
      → resolve_edge → GraphEdge::AsyncEnqueue { ring: quil_pd.message_ring }
      → traverse_edge → (*quil_pd.message_ring).enqueue(IpcCall { func_id:0x202, arg0:scancode, arg1:value, arg2:EV_KEY, caller_pd:shell_pd_id })
      → returns Ok(0) → shell sees (status=0, value=0)
```

### Kernel Receive Path (syscall 28, slot=0)
```
pdx_listen_raw(0) → kernel syscall 28, rdi=0
  1. lock current_pd.incoming_replies
  2. if !replies.is_empty() → pop → return (0x1, 1, reply.value, 0, 0)
  3. else → (*current_pd.message_ring).dequeue()
       IpcCall { func_id, arg0, arg1, arg2, caller_pd } → (func_id, caller_pd, arg0, arg1, arg2)
       empty → (0, 0, 0, 0, 0)
```

### Quil Boot Proof (pdx_call_and_reply — THE PROBLEM)
```
[quil.ready] (line 787)
  ↓
quil_save()  (legacy save/load proof, lines 855-875, always runs)
  → pdx_storage_call(OP_RAMFS_WRITE, ...)
    → pdx_call_and_reply(SLOT_STORAGE, OP_RAMFS_WRITE, ...)  (quil:464)
        pdx_call(SLOT_STORAGE, ...) → fire to sexfiles ring
        loop {                                                 (quil:470)
            msg = pdx_listen_raw(0)  ← DEQUEUES FROM OWN RING
            if msg.type_id == 0x1 { return }  ← waiting for sexfiles reply
            // ← SHELL SENDS OP_HID_EVENT (0x202) HERE
            // type_id=0x202 ≠ 0x1
            serial_println!("[quil.sync.skip] type_id=0x202")  ← CONSUMED AND LOST
        }
```

### Quil Main Loop (reached AFTER messages are gone)
```
[after boot proofs complete]
loop {
    msg = pdx_listen_raw(0)   ← ring now EMPTY (HID events consumed above)
    // pdx_listen_raw loops indefinitely, waiting for next message
    // [quil.pdx.listen] never fires because pdx_listen_raw never returns
}
```

---

## Routing / Capability Table

| Direction | Slot | Cap Type | Target | Granted By | Status |
|-----------|------|----------|--------|------------|--------|
| shell → quil | SLOT_QUIL=11 | Domain(quil_pd_id) | Quil PD message_ring | kernel init:405 | **CORRECT** |
| quil → sexdisplay | SLOT_DISPLAY | Domain(sexdisp_id) | sexdisplay | kernel init:420 | CORRECT |
| quil → sexfiles | SLOT_STORAGE | Domain(sexfiles_id) | sexfiles | kernel init:427 | CORRECT |
| quil → shell | (none) | N/A | N/A | N/A | NOT GRANTED |

**Key gap**: Quil has no capability to send signals back to shell. No reverse-ready notification possible via PDX without a new capability grant. STOP FIRST boundary — do not add capability without kernel STOP FIRST.

---

## Send/Receive Semantics

| Call | Syscall | Blocking | Returns | Notes |
|------|---------|----------|---------|-------|
| `pdx_call(slot, opcode, ...)` | 0 | No | (status, value) | AsyncEnqueue to target ring |
| `pdx_listen_raw(slot=0)` | 28 | Yes (loops until non-empty) | PdxMessage | Checks incoming_replies first, then message_ring |
| `pdx_reply(pd, value)` | 29 | No | u64 | Places into target incoming_replies |

**`status=0` from `pdx_call` means**: Message was successfully enqueued in target's `message_ring`. Does NOT mean delivered or received.

**`incoming_replies` priority**: Kernel syscall 28 checks replies BEFORE message_ring. A HID event (type_id=0x202) from shell lands in `message_ring`, NOT in `incoming_replies`. So it competes with storage replies for dequeue order.

**`pdx_call_and_reply` assumption (INCORRECT)**: Comment at quil:461 says "During boot proof, no HID events or pings will interfere." This assumption breaks when shell routes keys to Quil while Quil is in the boot-proof blocking window.

---

## Root Cause

**PRIMARY (HIGH CONFIDENCE)**: Quil's `pdx_call_and_reply` boot-proof skip loop (quil:470-477) dequeues and discards HID events (type_id=0x202) that arrive while Quil is waiting for a SLOT_STORAGE reply. The messages are consumed from the ring and logged as `[quil.sync.skip] type_id=0x202` (or silently dropped if budget=0). After the storage reply arrives and boot proofs complete, Quil enters its main loop. The ring is now empty. No further HID events arrive (user stopped typing). Quil's main-loop `pdx_listen_raw` blocks indefinitely.

**SECONDARY (MEDIUM)**: Quil stuck in `pdx_call_and_reply` forever because sexfiles never replies. Main loop never reached. This would also produce count=0 but would differ: `[quil.save_load.proof.save_ok]` would NOT appear in log. If sexfiles is not running, `pdx_call(SLOT_STORAGE, ...)` returns `ERR_CAP_INVALID` immediately — no blocking. If sexfiles IS running but hangs, blocking is possible.

**NOT THE CAUSE**:
- Kernel routing: CORRECT (cap grant at init:405, AsyncEnqueue path verified)
- Shell send: CORRECT (SLOT_QUIL=11 properly resolves to Quil PD, status=0 confirmed)
- Quil receive handler: CORRECT (OP_HID_EVENT branch at quil:907 matches correctly)
- Ring pointer: CORRECT (same `message_ring` used for enqueue and dequeue)

---

## Smallest Safe Fix

### Phase 1 — HID Event Stash in Boot-Proof Window

**File**: `servers/quil/src/main.rs` only.

**Mechanism**: Add a small static stash buffer. In `pdx_call_and_reply`'s skip loop, stash incoming HID events instead of discarding. After main loop starts, replay stash before entering `pdx_listen_raw` for the first time.

**Implementation sketch** (~20 lines):

```rust
// Static stash: scancode|value pairs buffered during boot-proof window.
const STASH_CAP: usize = 8;
static mut HID_STASH: [(u64, u64); STASH_CAP] = [(0, 0); STASH_CAP];
static mut HID_STASH_LEN: usize = 0;

// In pdx_call_and_reply skip loop (after line 474):
if msg.type_id == OP_HID_EVENT {
    unsafe {
        if HID_STASH_LEN < STASH_CAP {
            HID_STASH[HID_STASH_LEN] = (msg.arg0, msg.arg1);
            HID_STASH_LEN += 1;
            serial_println!("[quil.sync.stash] scancode={:#x} len={}", msg.arg0, HID_STASH_LEN);
        } else {
            serial_println!("[quil.sync.stash.drop] scancode={:#x} reason=full", msg.arg0);
        }
    }
}
```

```rust
// Before first pdx_listen_raw in main loop (after line 882, before line 884):
unsafe {
    for i in 0..HID_STASH_LEN {
        let (scancode, value) = HID_STASH[i];
        serial_println!("[quil.stash.replay] i={} scancode={:#x} val={}", i, scancode, value);
        // Call existing key handler inline (extract from match arm or duplicate).
        handle_key_event(scancode, value, &mut selected_row, &mut palette_active);
    }
    HID_STASH_LEN = 0;
}
```

Note: requires extracting the OP_HID_EVENT handler body into a function `handle_key_event(...)`.

**Patch size**: ~25 lines in quil/src/main.rs. No other files.

---

## STOP FIRST Boundaries

| Change | Required For | STOP FIRST |
|--------|-------------|------------|
| Kernel ring semantics | Guaranteed delivery without stash | YES |
| New SLOT (quil→shell ready signal) | Quil notifying shell when ready | YES (new cap grant = kernel init change) |
| `pdx_call_and_reply` changes in sex-pdx | ABI | YES |
| `pdx_listen_raw` changes in sex-pdx | ABI | YES |
| Ring thread-safety changes | Alternative delivery fix | YES |
| Stash buffer in quil/src/main.rs | Phase 1 fix | **NO** |
| Extracting `handle_key_event` fn in quil | Phase 1 fix | **NO** |

**Phase 1 fix has NO STOP FIRST boundaries.** Pure Quil userland change.

---

## Additional Diagnostic Needed (before implementing)

Verify root cause is PRIMARY hypothesis, not SECONDARY (stuck forever):

Add to boot proof area after `[quil.ready]`:
```
[quil.save_load.proof.start]   ← already present
[quil.save_load.proof.save_ok] ← appears if quil_save() returns Ok (sexfiles replied)
[quil.save_load.proof.save_fail] error=N  ← appears if quil_save() errors
```

Also add:
```
[quil.sync.skip] type_id=0x202  ← appears if HID consumed in skip loop
```

If `[quil.sync.skip] type_id=0x202` appears in next log: PRIMARY confirmed → Phase 1 fix safe.
If `[quil.save_load.proof.save_ok]` NOT present and Quil never exits proof: SECONDARY → investigate sexfiles.

---

## Future Proof Markers

```
[quil.sync.stash] scancode=N len=N       ← HID event captured during boot proof
[quil.sync.stash.drop] ... reason=full   ← stash overflow (increase STASH_CAP)
[quil.stash.replay] i=N scancode=N val=N ← stash replayed into main loop handler
[quil.stash.replay.done] count=N         ← all stashed events processed
[quil.pdx.listen] type_id=N             ← main loop receiving (existing marker)
[quil.key.recv] scancode=N val=N        ← key delivery confirmed (existing marker)
```

---

## Acceptance Criteria

1. `[quil.pdx.listen]` count ≥ 1 after user presses key while Quil is focused.
2. `[quil.key.recv]` count ≥ 1 per user keypress.
3. `[quil.stash.replay]` appears if HID events were stashed during boot proof.
4. No regression in Quil surface rendering, palette, or storage proof.
5. No kernel faults.
6. Shell `[silk-shell.key.route.send]` continues to appear (send side unchanged).

---

## Non-Goals

- Fixing delivery guarantees for ALL async PDX sends (kernel concern)
- Adding quil→shell reverse notification capability (requires kernel STOP FIRST)
- Making `pdx_call_and_reply` universally safe against HID interference (sex-pdx STOP FIRST)
- Rewriting Quil's boot sequence
- Eliminating the save/load proof

---

## Future Implementation Prompt

```
MISSION: QUIL_KEYBOARD_DELIVERY_PHASE1_V1

Root cause: Quil's pdx_call_and_reply boot-proof skip loop (quil:470-477) consumes
and discards OP_HID_EVENT messages that arrive during Quil's save/load boot proof.
After proof completes, main loop finds ring empty. See QUIL_KEYBOARD_DELIVERY_STOP_FIRST_REVIEW_V1.md.

File: servers/quil/src/main.rs ONLY.

Step 1: Add static stash near top of file (after static globals):
  static mut HID_STASH: [(u64, u64); 8] = [(0,0); 8];
  static mut HID_STASH_LEN: usize = 0;
  const STASH_CAP: usize = 8;

Step 2: In pdx_call_and_reply skip loop (line ~474-476), replace:
  serial_println!("[quil.sync.skip] type_id={:#x}", msg.type_id);
  with:
  if msg.type_id == OP_HID_EVENT {
      unsafe {
          if HID_STASH_LEN < STASH_CAP {
              HID_STASH[HID_STASH_LEN] = (msg.arg0, msg.arg1);
              HID_STASH_LEN += 1;
              serial_println!("[quil.sync.stash] scancode={:#x} len={}", msg.arg0, HID_STASH_LEN);
          } else {
              serial_println!("[quil.sync.stash.drop] scancode={:#x} reason=full", msg.arg0);
          }
      }
  } else {
      serial_println!("[quil.sync.skip] type_id={:#x}", msg.type_id);
  }

Step 3: Extract OP_HID_EVENT handler body from main loop into:
  fn handle_quil_key(scancode: u64, value: u64, selected_row: &mut u8, palette_active: &mut bool)
  (or inline the stash replay if handler is small enough)

Step 4: Before main loop (before `loop {` at line 884), add stash replay:
  unsafe {
      let n = HID_STASH_LEN;
      HID_STASH_LEN = 0;
      for i in 0..n {
          let (sc, val) = HID_STASH[i];
          serial_println!("[quil.stash.replay] i={} scancode={:#x} val={}", i, sc, val);
          handle_quil_key(sc, val, &mut selected_row, &mut palette_active);
      }
      if n > 0 { serial_println!("[quil.stash.replay.done] count={}", n); }
  }

Step 5: Emit [quil.pdx.listen] proof marker (already at line 892 — confirm it fires).

BACKUP before edit.
NO kernel changes. NO sex-pdx changes. NO shell changes.
STOP FIRST if: any change touches pdx_call_and_reply in sex-pdx, kernel, or shell.
```

---

## Files Changed

`docs/handoff/QUIL_KEYBOARD_DELIVERY_STOP_FIRST_REVIEW_V1.md` — created (this document).
No source files modified.

## Related Handoffs

- `docs/handoff/QUIL_KEYBOARD_DELIVERY_FIX_V1.md` — prior STOP FIRST attempt (runtime evidence)
- `docs/handoff/QUIL_KEYBOARD_BUFFER_NAV_V1.md` — original keyboard nav attempt
