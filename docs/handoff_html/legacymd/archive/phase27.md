```markdown
# Phase 27: The Transactional Capability Machine (CMPXCHG16B)

**Architectural Classification:** Transaction-ID driven capability system with single-writer CAS state, async recovery queuing, and generation-only reclamation fencing.

This phase officially transitions SexOS from a "safe concurrent router" to a hardware-atomic capability transition machine. We collapse identity verification, reservation, and intent binding into a single 128-bit atomic transition, explicitly separating execution and failure recovery into isolated, deterministic domains.

---

## 1. The Capability Transaction Descriptor (CTD)

The state machine expands to 128 bits to allow an atomic hardware transition (`lock cmpxchg16b` on x86_64) that binds logical intent to capability state.

```rust
#[repr(C, align(16))]
pub struct CapabilityTransactionDescriptor {
    /// Control Plane: [ in_flight (15 bits) | alive (1 bit) | generation (48 bits) ]
    pub control: u64,
    
    /// Intent Plane:  [ tx_id (32 bits) | opcode (16 bits) | payload (16 bits) ]
    pub intent: u64, 
}
```

* **No CPU Affinity:** Correctness depends entirely on the globally unique `tx_id`. Context migration, kernel preemption, or execution on sibling hyperthreads no longer breaks recovery semantics.
* **Atomic Binding:** The `intent` is mathematically fused to the exact `generation` at the instant of execution reservation.

---

## 2. Single-Writer IPC CAS Gate

The `compare_exchange16b` acts as the sole authority boundary. Execution is not physically embedded in the CAS; it is a strictly deterministic post-CAS consequence.

```rust
loop {
    let current_control = pd.control.load(Ordering::Acquire);
    let state = DomainState(current_control);
    
    if !state.is_alive() { return PDX_ERR_DENIED; }
    if state.in_flight_count() >= INFLIGHT_MAX { return PDX_ERR_BUSY; }

    // Mask-based field mutation (No bit-bleed arithmetic)
    let next_control = state.with_incremented_inflight().0;
    
    // Construct intent uniquely tied to this specific execution
    let next_intent = Intent::new(new_tx_id(), opcode, payload).0;

    match cmpxchg16b(
        &pd.ctd, 
        current_control, current_intent, // Expected
        next_control, next_intent        // Next
    ) {
        Ok(_) => {
            // COMMIT SUCCESS: State transition is formalized.
            execute_deferred(next_intent); 
            return PDX_OK;
        }
        Err((updated_ctrl, updated_int)) => {
            current_control = updated_ctrl;
            current_intent = updated_int;
            core::hint::spin_loop();
        }
    }
}
```

---

## 3. Asynchronous Fault Recovery Queue

The Ring-0 IDT (Interrupt Descriptor Table) handler is stripped of its ability to mutate CTD state directly. This eliminates multi-writer CAS collision and async state corruption during fault storms.

**The Recovery Flow:**
1.  **Fault:** A `#PF` or `#GP` occurs during a deferred execution sequence.
2.  **Emission:** The IDT handler catches the fault, extracts the active `tx_id`, and pushes a `RecoveryEvent { tx_id, reason }` to a lock-free queue. It then tears down the local thread.
3.  **Resolution:** A dedicated kernel recovery worker reads the queue and performs a formal, single-writer CAS to decrement the `in_flight` counter and neutralize the stranded `tx_id`.

---

## 4. Generation-Only Allocator Reclamation

The memory allocator must never interpret the full 128-bit CTD, as partial structure reconstruction creates allocator-level ABA vulnerabilities. Reclamation is strictly locked behind a monotonic generation fence.

**Reclamation Gate:**
```rust
// Allocator ONLY verifies that the state is logically dead and execution is drained.
// It does NOT verify the intent field, isolating it from IPC semantics entirely.

if current_generation == observed_generation && in_flight == 0 {
    // Zero-ABA memory recycle safe to proceed
}
```

---

## 5. Formal Invariants & Contract

1.  **Single-Phase Intent:** Execution intent is bound atomically with capability validation. There are no "reserved but empty" states.
2.  **Isolated Mutation Authority:** * *IPC Path:* Owns `intent` generation and `in_flight` increments.
    * *Recovery Worker:* Owns `in_flight` decrements on failure.
    * *Allocator:* Owns `generation` increments on slot reuse.
3.  **Execution is a Consequence:** `CMPXCHG16B` formalizes the right to execute. Execution is a guaranteed-observable side effect *only* upon CAS success.

*(End of Phase 27 Specification)*
```
```
```
```
```
``````markdown
# Phase 27: Transactional Capability Machine

**Classification:** Transaction-ID Driven Capability System with Single-Writer CAS State, Async Recovery Queuing, and Generation-Only Reclamation Fencing.

## 1. Capability Transaction Descriptor (CTD)
128-bit atomic capability state using `CMPXCHG16B`. Binds identity, liveness, concurrency, and execution intent into a single indivisible object.

```rust
#[repr(C, align(16))]
pub struct CapabilityTransactionDescriptor {
    // Control Plane: Tracks lifecycle and concurrency
    // [ in_flight (15 bits) | alive (1 bit) | generation (48 bits) ]
    pub control: u64,
    
    // Intent Plane: Tracks execution payload and global transaction identity
    // [ tx_id (32 bits) | opcode (16 bits) | payload (16 bits) ]
    pub intent: u64, 
}
```

## 2. Single-Writer CAS Gate
Validation, reservation, and intent binding occur in one atomic transition. Execution is a deferred consequence of a successful CAS.

```rust
loop {
    let current_ctrl = pd.control.load(Ordering::Acquire);
    let state = DomainState(current_ctrl);
    
    if !state.is_alive() { return PDX_ERR_DENIED; }
    if state.in_flight_count() >= INFLIGHT_MAX { return PDX_ERR_BUSY; }

    let next_ctrl = state.with_incremented_inflight().0;
    let next_intent = Intent::new(generate_tx_id(), opcode, payload).0;

    match cmpxchg16b(
        &pd.ctd, 
        current_ctrl, current_intent,
        next_ctrl, next_intent
    ) {
        Ok(_) => {
            // State bound atomically. Defer execution.
            execute_pdx_payload(next_intent); 
            return PDX_OK;
        }
        Err((updated_ctrl, updated_int)) => {
            current_ctrl = updated_ctrl;
            current_intent = updated_int;
            core::hint::spin_loop();
        }
    }
}
```

## 3. Asynchronous Fault Recovery
The IDT never mutates the CTD directly, preserving single-writer CAS discipline and preventing async state corruption.

1. **Fault Intercept:** `#PF` or `#GP` occurs during execution.
2. **Event Emission:** IDT halts unwind, extracts `tx_id` from the faulting thread's context, and pushes `RecoveryEvent { tx_id, reason }` to a lock-free queue.
3. **Async Resolution:** A dedicated kernel recovery worker drains the queue and executes a compensating CAS to decrement `in_flight` and neutralize the specific `tx_id`.

## 4. Generation-Only Allocator Fence
The allocator never performs a 128-bit structural CAS on the CTD to prevent multi-field semantic ABA. Memory reclamation relies strictly on monotonic generation fencing.

```rust
// Allocator observes logical death and concurrency drain
if pd.generation == expected_generation && pd.in_flight == 0 {
    // Safe to reclaim and recycle memory slot
}
```

## 5. Formal System Invariants
1. **Execution is Post-Commit:** `CMPXCHG16B` guarantees state transition atomicity, not execution atomicity.
2. **Transaction Identity:** Correctness relies on globally unique `tx_id`s, not `cpu_id` or core affinity. Transactions safely survive scheduler preemption and core migration.
3. **Segmented Mutation Authority:**
   * **IPC Hotpath:** Increments `in_flight`, mutates `intent`.
   * **Recovery Worker:** Decrements `in_flight` on execution fault.
   * **Allocator/Revocation:** Mutates `alive` and `generation`.
```
