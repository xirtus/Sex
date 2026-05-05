# BELL_LIST_SUMMARY_IMPLEMENT_V1

**Status:** Implemented. Code changed. Build passes (kernel + sexbell).
**Date:** 2026-05-05
**Depends on:** `BELL_LIST_SUMMARY_PLAN_V1.md`, `BELL_RAM_QUEUE_FREEZE_V1.md`

---

## 1. Files Changed

| File | Change | Type |
|------|--------|------|
| `servers/sexbell/src/main.rs` | Add OP_BELL_LIST handler with markers | Server edit |
| `kernel/src/init.rs` | Temporary proof scaffold (removed after proof) | Scaffolding |
| `docs/handoff/BELL_LIST_SUMMARY_IMPLEMENT_V1.md` | This document | Handoff |

**Not changed:** sex-pdx (OP_BELL_LIST=0xC3 already assigned), silk-shell, sexdisplay, SilkBar, storage, limine.cfg, sexos_build_spec.toml

---

## 2. Argument Parsing

### arg0 layout

| Bits | Field | Validation |
|------|-------|------------|
| 0-7 | `lane_filter` | 0xFF (all) or 0..=5 (specific lane) |
| 8-15 | `max_results` | 1..=4 only (reject 0 or >4, no clamping) |
| 16-63 | `_reserved` | 0 (ignored) |

### Validation rules

- `lane_filter != 0xFF && lane_filter > 5` → `[bell.list.reject] reason=invalid_lane`
- `max_results == 0 || max_results > 4` → `[bell.list.reject] reason=invalid_count`
- On reject: emit marker, `continue` (no queue access)

Per user adjustment: **max_results is 1..=4, reject >4** — no clamping as originally proposed in the plan.

---

## 3. Queue Read Behavior

### Newest-first iteration

The handler reads the queue in newest-first order (reverse ring order):

```rust
for i in 0..BELL_QUEUE.count as usize {
    let idx = (BELL_QUEUE.tail as usize + BELL_QUEUE_CAPACITY - 1 - i) % BELL_QUEUE_CAPACITY;
    let entry = &BELL_QUEUE.entries[idx];
    // match filter, emit marker, count
}
```

### Lane filter

- `lane_filter == 0xFF`: matches all entries
- `lane_filter == entry.final_lane`: matches entries with that lane
- Non-matching entries are skipped (no marker emitted)

### Max results

- `match_count >= max_results`: stops iteration early
- Returns up to `max_results` matching entries

### Queue mutation

**None.** The handler only reads `BELL_QUEUE.count`, `BELL_QUEUE.tail`, and `BELL_QUEUE.entries[idx]` fields. No `push`, `pop`, `remove`, `clear`, `insert`, or `delete` operations. The queue is left unchanged after a list operation.

---

## 4. Marker-Only Strategy

| Marker | Budget | Fields | When |
|--------|--------|--------|------|
| `[bell.list.recv]` | 8 | `lane_filter`, `max_results`, `caller_pd` | Valid request received |
| `[bell.list.item]` | 16 | `event_id`, `final_lane`, `category`, `privacy`, `redaction` | Per matching entry |
| `[bell.list.empty]` | 4 | — | Queue has 0 matching entries |
| `[bell.list.done]` | 8 | `count` | After all item markers emitted |
| `[bell.list.reject]` | 4 | `reason`, `caller_pd` | Invalid lane_filter or max_results |

### Marker budget justification

| Marker | Budget | Expected per boot | Safety margin |
|--------|--------|-------------------|---------------|
| recv | 8 | 1 (single scaffold call) | 8x |
| item | 16 | 0-16 (queue capacity) | 1x (caters to full queue) |
| empty | 4 | 0-1 | 4x |
| done | 8 | 0-1 | 8x |
| reject | 4 | 0 (valid request) | 4x (for error cases) |

### No reply path

`pdx_reply` is NOT called. The kernel scaffold does not expect a reply. Marker-only proof verifies the handler works. Real reply path deferred to when a real sender (SilkBar) is wired.

---

## 5. Kernel Scaffold

### Location

`kernel/src/init.rs`, after sexbell self-cap grant (line 177), before framebuffer handoff (line 179).

### Code

```rust
// ── BELL_LIST_SUMMARY_PROOF_SCAFFOLD ──
// REMOVAL PROMISE: Temporary proof scaffold for OP_BELL_LIST.
// Removed in BELL_LIST_SUMMARY_CLEANUP_V1 after QEMU proof.
if sexbell_id != 0 {
    use crate::ipc::DOMAIN_REGISTRY;
    use crate::ipc::messages::MessageType;

    let arg0: u64 = (0xFFu64 << 0) | (4u64 << 8); // lane_filter=0xFF, max_results=4
    let msg = MessageType::IpcCall {
        func_id:   sex_pdx::OP_BELL_LIST,  // 0xC3
        arg0,
        arg1:      0,
        arg2:      0,
        caller_pd: 0,
    };
    if let Some(pd) = DOMAIN_REGISTRY.get(sexbell_id) {
        unsafe { let _ = (*pd.message_ring).enqueue(msg); }
        serial_println!("[kernel.sexbell.list.test] enqueued OP_BELL_LIST to sexbell");
    }
}
```

### Cleanup guarantee

~18 lines, removed in BELL_LIST_SUMMARY_CLEANUP_V1 after QEMU proof. Marked with `REMOVAL PROMISE` comment.

### Behavior at boot

The scaffold sends OP_BELL_LIST with lane_filter=0xFF, max_results=4. Since the queue is empty at boot (no OP_BELL_NOTIFY has been sent), the handler will count 0 matches and emit `[bell.list.empty]`. This is acceptable for the implementation phase. The proof phase may add a notify scaffold to populate the queue first.

---

## 6. Forbidden Features — Confirmed Absent

| Feature | Check | Result |
|---------|-------|--------|
| Reply path (`pdx_reply`/`pdx_call`) | `rg` on sexbell/main.rs | ❌ Absent |
| Heap/alloc/Vec/String/Box | `rg` on sexbell/main.rs | ❌ Absent |
| SilkBar integration | `rg` on sexbell/main.rs | ❌ Absent |
| Storage/persistence | `rg` on sexbell/main.rs | ❌ Absent |
| Private content (title/body/sender) | `rg` on sexbell/main.rs | ❌ Absent |
| Queue mutation during list | `rg` on list handler | ❌ Absent |
| sex-pdx edits | No changes | ❌ Absent |
| Cap grants to external senders | `rg "SLOT_BELL.*Domain" init.rs` | ❌ Only self-cap |
| max_results clamping | Implementation uses reject-not-clamp | ✅ Correct |

---

## 7. Build Result

- `sex-kernel` (lib): ✅ Compiles (0 errors, pre-existing warnings only)
- `sexbell` (bin): ✅ Check passes (0 errors, only linker error from pre-existing toolchain `memcmp` issue)
- `silk-shell`: ❌ Pre-existing error (`toggle_spindle` not found) — unrelated to this change

---

## 8. Next Phase

**BELL_LIST_SUMMARY_PROOF_V1** — QEMU boot proof showing `[bell.list.*]` markers. May add notify+list kernel scaffolds for a populated-queue proof.

## 9. Verification

1. `rg -n "OP_BELL_LIST|bell.list|kernel.sexbell.list.test" servers/sexbell/src/main.rs kernel/src/init.rs` — all markers present
2. `rg -n "pdx_reply|Vec|String|alloc|silkbar|storage|clear|mutate" servers/sexbell/src/main.rs` — forbidden features absent
3. `cargo check -Zbuild-std=core,alloc --target x86_64-sex.json -p sexbell` — compiles ✅
4. `rg -n "push|pop|remove|clear|insert|delete|write" servers/sexbell/src/main.rs | rg -i list` — no queue mutation ✅

---

*End of BELL_LIST_SUMMARY_IMPLEMENT_V1.md*
