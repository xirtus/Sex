# E9_PRE_DURABLE_STORAGE_SAFETY_AUDIT_V1

**Status:** Report only. No code changed. No architecture redesign.

**Date:** 2026-05-05

**E9 Gate:** Decides whether durable-storage planning (E9+) may proceed safely.

---

## Executive Summary

**Verdict: GO — no critical or high findings that block E9.**

The audit inspected the sexstore/storage path, PDX caller_pd/capability integrity, kernel init/cap grant path, and MPK/PKU isolation as relevant to durable storage safety. All findings are **MEDIUM** or **LOW** severity. No CRITICAL or HIGH issues found.

| Severity | Count | E9 Gate |
|----------|-------|---------|
| CRITICAL | 0 | Block E9 |
| HIGH     | 0 | Block E9 |
| MEDIUM   | 3 | Document — does not block E9 |
| LOW      | 3 | Acknowledge — no action required |

**The kernel's caller_pd chain is authoritative.** `PdxMessage.caller_pd` is set by the kernel at `syscalls/mod.rs:255` from the `IpcCall` message created at `ipc.rs:180-181`, which uses `current_pd.id` (`ipc.rs:193`). No userspace code paths can falsify caller identity. sexstore's capability gate (`store_cap_allowed`) correctly restricts to shell-only (domain 3) on the shell range (0x01–0x0F).

**Storage path is safe for E9 planning.** The sexstore single-threaded dispatch model prevents races. Proof markers log metadata only (no stored values). All marker types have per-boot budgets.

### E9 Readiness

| Requirement | Status |
|-------------|--------|
| Critical/high findings blocking E9? | **None** — GO |
| caller_pd integrity verified? | ✅ Kernel-authoritative, not user-falsifiable |
| sexstore capability gate verified? | ✅ Present on all 3 dispatch paths (PUT, GET, DEL) |
| Proof markers leak stored values? | ❌ No — verified by inspection |
| PKU isolation bypass risk? | ❌ No — sexstore has no PKU manipulation code |
| Reply misrouting risk? | ⚠️ MEDIUM — depth-1 reply buffer, no exploit path today |

---

## Findings

### MEDIUM-01: Reply buffer depth of 1 can silently drop replies

**File:** `kernel/src/capability.rs:247`, `kernel/src/ipc/router.rs:36-54`

**Description:** `ProtectionDomain::incoming_replies` is created with `VecDeque::with_capacity(1)` (line 247). `send_reply()` at router.rs:40-44 pushes a reply and pops front if `len >= 1`. If a server receives two replies before processing any, the older reply is silently dropped.

The sexstore protocol is synchronous — sexstore calls `kv_reply_status()` then loops back to `pdx_listen_raw()`. The caller (silk-shell) also follows a synchronous pattern (send → listen → process). However, the kernel does not enforce synchronous ordering. A fast caller could send two requests before the first reply is consumed.

```
Scenario:
1. silk-shell sends PUT(key=1, val=A) → sexstore processes, replies KV_OK
2. sexstore loops to pdx_listen_raw()
3. silk-shell sends PUT(key=1, val=B) before calling pdx_listen_raw()
4. Kernel enqueues two IpcCall messages in sexstore's message_ring
5. [sexstore never sees reply loss because it never has two pending operations]
   — BUT if reply arrives before sexstore calls pdx_listen_raw(), the depth-1 buffer drops it
```

**Risk:** In current synchronous protocol — LOW (no impacted path).  
**E9 Risk:** If durable storage adds async operations (background flush, write-back cache), reply loss becomes data loss. **MEDIUM for E9 planning.**

**Recommendation for E9:** Before adding any async storage operation, increase reply buffer depth to at least 8, or make it unbounded with backpressure. Add a `send_reply_or_notify()` variant that returns `Err(BufferFull)` instead of silently dropping.

---

### MEDIUM-02: Hardcoded KV_SHELL_CALLER domain ID (3)

**File:** `servers/sexstore/src/main.rs:111`

```rust
const KV_SHELL_CALLER: u64 = 3;
```

**Description:** The only authorized caller for sexstore operations is hardcoded as domain 3 (silk-shell). If silk-shell is ever spawned with a different PD ID (due to init order changes, multi-shell support, or testing), sexstore silently denies all operations. The caller sees `KV_DENIED` with no indication that the hardcoded constant is the root cause.

**Risk:** Brittle design. Currently harmless because `kernel/src/init.rs:39` uses a fixed spawn order where silk-shell always gets domain 3. Any change to the spawn order in `module_paths` would break storage silently.

**Recommendation for E9:** Before adding durable storage configuration, replace the hardcoded constant with either:
- A boot-time grant: kernel passes the authorized PD ID as an init argument
- A static capability table in sexstore (compile-time array of authorized PD IDs per key range, as originally proposed in E3 spec)

---

### MEDIUM-03: Reclaimed tombstoned slot keeps old generation counter

**File:** `servers/sexstore/src/main.rs:269-287`

**Description:** When sexstore inserts into a fresh empty slot (line 257), generation is set to 1. When it reclaims a tombstoned slot (line 278), it calls `bump_generation()` which increments from the old value. This means:

- Empty slot insert: gen = 1
- Update active key: gen += 1 (2, 3, 4, ...)
- DELETE → tombstone: gen += 1
- Reclaim same slot for different key: gen += 1 (continues from tombstone value)
- But reclaim doesn't reset the counter to 1 for the new key

```
Example:
  PUT(key=0x01, val=A) into empty slot → gen=1
  PUT(key=0x01, val=B) update → gen=2
  DEL(key=0x01) → tombstone → gen=3
  PUT(key=0x02, val=C) reclaims same slot → bump → gen=4 (not 1!)
```

**Risk:** None to security. Generation is only used for internal tracking and proof markers. No caller protocol depends on generation values. The generation counter can never reach 0 (wraps 255→1).

**Recommendation for E9:** If generation is ever exposed to callers (for optimistic concurrency or CAS operations), reset to 1 on reclaim for semantic clarity. Currently not required.

---

### LOW-01: PKU page table walk reads via HHDM without validation

**File:** `kernel/src/pku.rs:118-244` (`tag_virtual_address`, `set_page_user_accessible`)

**Description:** Both functions read page table entries via physical addresses translated through the HHDM direct-map offset. If any page table entry contains a corrupt physical address (e.g., bits 51:12 point to non-existent memory), the resulting `(phys_addr + hhdm_offset)` virtual address could be a dangling pointer.

**Risk:** LOW — page tables are only modified by kernel code during init (`pku.rs`, `memory/manager.rs`). No userspace input can trigger a malicious page table walk. A kernel memory-safety bug elsewhere could corrupt a page table entry and cause a crash in these functions, but that would be a different bug with its own severity.

**E9 Relevance:** Direct. If durable storage uses DMA or persistent memory mapped through page tables, this code path (or similar PT walks) must be hardened. For E9 (policy/gate docs only), no action needed.

---

### LOW-02: sexstore `caller_pd` widened from u32 to u64 and back

**File:** `servers/sexstore/src/main.rs:147`, `crates/sex-pdx/src/lib.rs:57`, `kernel/src/ipc/router.rs:36`

**Description:** The caller_pd type chain crosses three type boundaries:
1. Kernel `IpcCall` stores `caller_pd: u32` (`ipc/messages.rs:52`)
2. `PdxMessage.caller_pd: u32` (`sex-pdx/lib.rs:57`)
3. sexstore reads `msg.caller_pd as u64` (`main.rs:147`) for `kv_reply_status(caller, ...)`
4. Kernel `send_reply` takes `target_pd_id: u32` (`router.rs:36`)

The u64→u32 truncation in step 4 is safe because PD IDs are currently ≤ 32. If PD IDs ever exceed u32::MAX, the reply would silently target the wrong domain.

**Risk:** LOW — PD IDs are currently 1..32, well within u32 range. Type narrowing (u64 to u32) at the kernel boundary is explicit. No truncation can occur under current constraints.

**Recommendation for E9:** Add a `debug_assert!` in `send_reply` that `target_pd_id == (val as u32)` or similar to catch truncation during testing. Or change the syscall ABI to use u32 consistently.

---

### LOW-03: sexstore uses raw pointer access to KV table

**File:** `servers/sexstore/src/main.rs:209, 350, 447`

```rust
let kv_ptr: *mut KvSlot = core::ptr::addr_of_mut!(KV) as *mut KvSlot;
```

**Description:** sexstore accesses the static `KV` table through raw pointers cast from `addr_of_mut!()` inside `unsafe` blocks. The code is correct because sexstore is single-threaded (infinite loop at line 145, no thread spawning). However, there is no explicit synchronization primitive documenting this invariant.

**Risk:** LOW — sexstore has no concurrency. Adding a second thread or switching to async would cause data races without compiler warnings.

**E9 Relevance:** If durable storage adds background threads (flush, compaction, checkpoint), the raw pointer access pattern must be wrapped in a `Mutex` or use atomic operations. For E9 (policy/gate), no action needed.

---

## Clean Areas

The following areas were inspected and found to have **no issues** relevant to E9:

### sexstore dispatch paths (all 3: PUT, GET, DEL)

- ✅ `store_cap_allowed()` called on every dispatch path before any state mutation
- ✅ `key == 0` rejected as invalid on all 3 paths
- ✅ Value envelope validated on PUT path (magic 0xAC, version 0x01, XOR checksum)
- ✅ Slot index bounded to 0..15 (loop stops before `KV_SLOT_COUNT`)
- ✅ DELETE idempotent: active→tombstone (KV_OK), already tombstoned (KV_OK), missing (KV_NOT_FOUND)
- ✅ No stale value leak on reclaim: `val` is overwritten before state is set to active
- ✅ Generation never 0 after write: new slots start at 1, bump wraps 255→1

### Proof markers

- ✅ All 18 marker types have per-boot budgets
- ✅ No marker logs stored u64 values — verified by grep of all `serial_println!` calls
- ✅ No marker logs paths, document titles, or user content
- ✅ All markers correctly classified as StructuralMeta or PublicProof (per E8)
- ✅ Budgets decrement atomically (single-threaded, no race)

### PDX caller_pd chain

- ✅ `caller_pd` set by kernel at `syscalls/mod.rs:255` from `IpcCall.caller_pd`
- ✅ `IpcCall.caller_pd` set by `safe_pdx_call` at `ipc.rs:193` from `current_pd.id`
- ✅ Kernel-authoritative — no userspace code can falsify caller_pd
- ✅ sex-pdx exposes `caller_pd` as read-only field in `PdxMessage`

### Kernel cap grant path

- ✅ sexstore gets only `SLOT_SEXSTORE` capability (CapabilityData::Domain(sexstore_id)) at `init.rs:101`
- ✅ silk-shell gets `SLOT_SEXSTORE` for access at `init.rs:101`
- ✅ Boot init is monotonic: PDs created before scheduler, scheduler before userspace (init.rs:209-224)
- ✅ Boot phase transitions validated in `ipc.rs` BootController

### MPK/PKU isolation (storage-relevant)

- ✅ sexstore has no PKU manipulation code — zero calls to wrpkru/rdpkru/pku functions
- ✅ sexstore domain gets UNTRUSTED pkey (15) by default via `pd/create.rs`
- ✅ PKRU restored before iretq to userspace per `init.rs:234-265` (`jump_to_userland`)
- ✅ PKU_ENABLED atomic gate on all wrpkru calls prevents execution on non-PKU hardware

### Concurrency (storage-relevant)

- ✅ sexstore is single-threaded — no concurrent access to `KV` table
- ✅ `msg.caller_pd` is immutable for the duration of each dispatch iteration
- ✅ Proof marker budgets (`static mut u32`) are safe under single-threaded access

---

## Key Audit Priority Summary

| Priority | Area | Result |
|----------|------|--------|
| 1 | sexstore E4–E8 code (517 lines) | 3 MEDIUM findings, 1 LOW — no E9 blockers |
| 2 | PDX caller_pd / capability integrity | ✅ Clean — kernel-authoritative |
| 3 | Kernel init/cap grant path | 1 MEDIUM (hardcoded domain 3) |
| 4 | MPK/PKU isolation (storage-relevant) | 1 LOW (HHDM PT walk) |

---

## E9 Recommendations from Audit

These are not bugs — they are design constraints that E9 should account for:

1. **Reply buffer depth**: Before adding async storage operations, increase `incoming_replies` depth from 1 to at least 8, or add backpressure. See MEDIUM-01.

2. **Domain ID configuration**: Replace `KV_SHELL_CALLER = 3` with a boot-time grant or static table before durable storage adds configuration. See MEDIUM-02.

3. **Generation semantics**: Decide whether generation resets on reclaim. Currently not needed, but if generation is part of a future CAS protocol, the semantics must be defined. See MEDIUM-03.

4. **HHDM page table walk**: If durable storage maps persistent memory via page tables, the `pku.rs` walk functions should validate physical addresses before dereference. See LOW-01.

5. **Proof marker budgets for persistence**: Current budgets are per-boot (reset on power cycle). E9 must define whether markers are persisted across boots and how budgets translate to durable storage.

---

## Appendix: Files Inspected

| File | Lines | Relevance |
|------|-------|-----------|
| `servers/sexstore/src/main.rs` | 517 | Primary audit target — all E4–E8 storage code |
| `crates/sex-pdx/src/lib.rs` | 411 | PDX message definitions, caller_pd field, slot constants |
| `kernel/src/syscalls/mod.rs` | 414 | Syscall dispatch — caller_pd return at line 255 |
| `kernel/src/ipc.rs` | 226 | safe_pdx_call, traverse_edge — caller_pd origin at line 193 |
| `kernel/src/ipc/messages.rs` | 111 | IpcCall message type with caller_pd field |
| `kernel/src/ipc/router.rs` | 55 | send_reply — reply buffer depth check |
| `kernel/src/capability.rs` | 313 | ProtectionDomain, incoming_replies (line 247) |
| `kernel/src/pku.rs` | 258 | PKU page table walk, HHDM dereference |
| `kernel/src/init.rs` | 266 | Boot init, cap grants for sexstore (line 100-103) |
| `kernel/src/pd/create.rs` | (partial) | PD creation with PKU key assignment |
| `kernel/src/scheduler.rs` | 493 | Task/scheduler — concurrency context |
| `kernel/src/interrupts.rs` | 708 | IDT, interrupt safety |
| `kernel/src/ipc_ring.rs` | 69 | Lock-free ring buffer |
| `kernel/src/core_local.rs` | 113 | Per-CPU data |
| `kernel/src/smp.rs` | 19 | SMP primitives |
