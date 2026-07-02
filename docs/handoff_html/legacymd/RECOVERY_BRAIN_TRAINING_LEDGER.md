# RECOVERY_BRAIN_TRAINING_LEDGER

- date: 2026-05-08
- status: LIVING DOCUMENT — append new classes, never delete entries
- scope: docs only — no code, no ABI, no kernel changes

---

## Purpose

Recovery brain training ledger. Each entry teaches the next Claude session (or human
engineer) how to recognize, diagnose, and safely fix a known failure class in SexOS.

Format per entry:
- Symptom: what the developer sees
- Evidence pattern: exact serial log markers or fault text
- Likely cause: root mechanism
- Safe fix: minimal correct repair
- Forbidden fix: what looks tempting but breaks something else
- Runtime gate: which gate catches this failure class

---

## Failure Class 1 — PD_STACK_OVERFLOW

**Symptom:**
PD faults immediately or within first few ticks. Scrollback, history, or TUI state corrupted.
QEMU shows non-present page fault at address slightly below the mapped stack base.

**Evidence pattern:**
```
[fault.kill pd=spindle rip=0x... cr2=0x7ffe...]
#PF at cr2 below stack base
```
Write to non-present page just below mapped stack top.

**Likely cause:**
Large local variables or arrays placed on the stack in `_start` or a deeply called function.
The PD stack is 64 KiB by default. A single `ScrollbackState` or session history buffer
of 64+ KiB overflows it immediately.

**Real example:**
Spindle scrollback + history: ~115 KiB local state on 64 KiB PD stack → immediate fault.
Fix: moved `static mut SESSION_STATE: SpindleState` to BSS/static. Stack usage dropped to
a few hundred bytes. See commit `705b06c`.

**Safe fix:**
Move large state structs to `static mut` (BSS segment) or heap-allocated via PDX cap.
Verify: `size` on the ELF binary — BSS should grow, stack frame should shrink.

**Forbidden fix:**
- Do NOT increase the kernel stack allocation limit without STOP FIRST review.
- Do NOT split state across multiple small locals to "fit" — that just defers the fault.
- Do NOT use `Box` unless the allocator cap is proven available at the fault site.

**Runtime gate:**
FAULT_GATE. Any `fault.kill` or `#PF` → RED. Boot-step proof: `[pd.ready]` marker
after `_start` confirms stack survived initialization.

---

## Failure Class 2 — MARKER_DRIFT

**Symptom:**
Gate script reports marker absent. Serial log shows a marker that looks right but doesn't
match the expected string. Gate = RED despite PD appearing to run correctly.

**Evidence pattern:**
```
grep "[silkshell.ready]" → 0 results
grep "silkshell" → found "[silkshell.init.done]"
```
Marker emitted under a different name than the canonical alias.

**Likely cause:**
Developer changed the marker string during a refactor, or copy-pasted a marker from
another PD and forgot to update the name. Gate scripts and downstream waiters check
exact string equality.

**Safe fix:**
- Restore the canonical marker string (see BOOTGRAPH_READINESS_V1.md alias table).
- If the canonical string needs to change, update the alias table AND all gate scripts
  AND all downstream waiters in the same commit.

**Forbidden fix:**
- Do NOT update only the emitting PD without updating gate scripts — gate stays RED.
- Do NOT add a second marker alongside the wrong one "for compatibility" — two markers
  = two gates to maintain = future drift.

**Runtime gate:**
SPAWN_GATE (marker absent = PD not confirmed alive) or BOOTGRAPH gate (exact string check).

---

## Failure Class 3 — RUNTIME_UNREACHED_MARKER

**Symptom:**
Marker exists in source code. Gate script expects it. But it never appears in serial log.
PD may appear spawned (SPAWN_GATE passes) but subsequent markers absent.

**Evidence pattern:**
```
[kernel.spawn.sexfiles] id=11
(no [sexfiles.ready])
(no subsequent sexfiles markers)
```
PD spawned but crashed or deadlocked before reaching the marker emit site.

**Likely cause:**
- Stack overflow before `[pd.ready]` (see class 1).
- Null cap dereference during init (PDX slot not yet granted by kernel Phase 25).
- Panic in RamFS mount, NVMe init, or font load before marker.
- Infinite loop in initialization code.

**Safe fix — boot-step proof:**
Add intermediate markers at each major init step:
```
[sexfiles.init.start]
[sexfiles.ramfs.mount]
[sexfiles.nvme.probe]
[sexfiles.ready]
```
Run gate. First absent marker = exact crash site. Fix that site.

**Safe fix — first-fatal-fault triage:**
```
grep -E "fault|panic|#PF|#GP|FATAL" serial.log | head -5
```
Identify the fault. Fault address + RIP → source location via `addr2line`.

**Forbidden fix:**
- Do NOT move the `[pd.ready]` marker earlier to make the gate pass — it must be the
  last thing before `pdx_recv`, not a lie.
- Do NOT disable FAULT_GATE to hide the fault.

**Runtime gate:**
BOOTGRAPH gate (ready marker absent). FAULT_GATE (if fault precedes marker).

---

## Failure Class 4 — CLOCK_SOURCE_ARBITRATION

**Symptom:**
Visible clock on silkbar is frozen. `[silkbar.clock.send]` count = 0 in gate results.
`CLOCK_GATE = FAIL`.

**Evidence pattern:**
```
silkbar.clock.send ticks: 0
SetClock rejected: stale tick gate
```
SetClock call arrives but is dropped by a staleness guard that considers the new tick
older than the last accepted tick (e.g. producer restarted and counter reset).

**Likely cause:**
Clock producer (e.g. kernel timer or a PD clock arm) restarts and sends tick=1.
Receiver has last_tick=500. Guard rejects tick=1 as stale.
This is correct behavior for replay protection but breaks producer restart.

**Real example:**
CLOCK_FREEZE_FALLBACK_GATE_V1 — silkbar stale-time gate rejected SetClock after
QEMU clock source changed. Fix: accept if new producer state differs from last known
producer identity, OR accept if tick gap > threshold.

**Safe fix:**
Allow SetClock to reset the stale guard when the producer epoch changes.
Or: relax the stale guard to accept tick=1 after a gap of N ticks.
Gate proof: `[silkbar.clock.send]` ≥ 2 within 900-second probe window.

**Forbidden fix:**
- Do NOT remove the stale guard entirely — it prevents replay attacks from buggy producers.
- Do NOT increase the probe window past 900s — that just hides latency, not fix.

**Runtime gate:**
CLOCK_GATE (`[silkbar.clock.send]` ≥ 2).

---

## Failure Class 5 — RENDER_STARVATION

**Symptom:**
Display updates cause CPU to spin. Many incoming messages each trigger a full redraw.
Other PDs starve. Timer ticks pile up. Scheduler appears stuck on sexdisplay.

**Evidence pattern:**
```
task.running pd=sexdisplay count=500+
task.running pd=silk-shell count=11
[sexdisplay.render.start] repeated >100 per second
```
Render loop runs once per message, never yields, other PDs get no scheduler time.

**Likely cause:**
`pdx_recv` loop calls `render()` immediately on every message, including redundant
intermediate states. No drain window, no coalescing, no yield.

**Real example:**
`sexdisplay` coalescing fix — bounded drain (≤16 messages per render cycle) +
coalesced redraw (one render per drain cycle) + explicit `pdx_yield` after render.
See commit `89d9ecc`.

**Safe fix:**
1. Bounded drain: read up to N messages per loop iteration, break after N.
2. Coalesced redraw: set a `dirty` flag per message, render once after drain.
3. Yield: call `pdx_yield` after each render to give other PDs scheduler time.
N = 16 is a proven safe value for current message volumes.

**Forbidden fix:**
- Do NOT remove the render call entirely — that causes display freeze (opposite problem).
- Do NOT increase N beyond 64 without a new render budget proof.
- Do NOT add a sleep — use `pdx_yield` to yield cooperatively.

**Runtime gate:**
SCHED_GATE (all PDs must have `task.running` ≥ 1). Budget proof markers at render entry.

---

## Failure Class 6 — PDX_RECEIVER_NOT_READY

**Symptom:**
Message sent to a PD that hasn't started its receive loop yet. Message dropped silently,
or PDX slot returns error, or sender faults on null cap.

**Evidence pattern:**
```
[sexusb.send.sexinput] (appears before [sexinput.ready])
(no sexinput response)
```
Sender emits before receiver's `pdx_recv` loop is live.

**Likely cause:**
Boot order: sexusb starts xHCI ring fast, sexinput takes longer to init HID table.
No handshake — sexusb fires immediately after cap grant.

**Safe fix (V1):**
Sender defers first call until receiver's readiness marker confirmed in serial log.
In production: sender checks marker via boot-step proof before first call.

**Safe fix (V2):**
Sender sends `OP_PING` to receiver slot. Defers all calls until `OP_PONG` received.
Timeout after N ticks → log `[boot.wait.timeout]` → continue degraded.

**Forbidden fix:**
- Do NOT add `pdx_yield` spin-loop waiting for receiver — starvation risk.
- Do NOT lengthen sender initialization arbitrarily to "give receiver time" — fragile.

**Runtime gate:**
BOOTGRAPH gate (sender marker must not precede receiver ready marker in log order).

---

## Failure Class 7 — PKU_CROSS_PD_ACCESS

**Symptom:**
PD faults accessing memory it doesn't own. Protection key violation.
Fault address belongs to a different PD's mapped region.

**Evidence pattern:**
```
#PF at cr2=0x... (address in sexdisplay framebuffer region)
fault in silk-shell context
PKU violation: key=2 not open for pid=3
```
Cross-PD pointer dereference — one PD holds a raw pointer into another PD's address space.

**Likely cause:**
Shared pointer passed via PDX message (wrong: passes address, not capability).
Or: stale pointer from a PD that was destroyed and re-spawned at different address.

**Safe fix:**
- Identify the owning PD for the target address (check ELF load address + size).
- Route all cross-PD data through PDX message or capability.
- Never pass raw pointers between PDs — pass indices, IDs, or capability references.
- If data must be shared, use a kernel-mediated shared capability (not a raw mapping).

**Forbidden fix:**
- Do NOT open the PKU key globally to "fix" the fault — that defeats PKU isolation.
- Do NOT map the target region into the faulting PD — breaks domain isolation invariant.

**Runtime gate:**
FAULT_GATE (any PKU fault → RED). No PKU violations tolerated in shipping builds.

---

## Failure Class 8 — ABI_OPCODE_DRIFT

**Symptom:**
PD sends a message. Receiver does not respond, or responds with wrong behavior.
No fault. Silent mismatch.

**Evidence pattern:**
```
[sexdisplay.recv opcode=0x12]  ← sender expected 0x14
(no [sexdisplay.surface.update] marker)
```
Sender and receiver have diverged on opcode value for the same operation.

**Likely cause:**
- New opcode added without updating both sender and receiver.
- Opcode constant defined in two places (copy-paste) and one was changed.
- Opcode inserted into enum, shifting all higher values.

**Safe fix:**
- Contract review: grep all opcode constants for the affected PD pair.
- Compare sender constant vs receiver match arm — they must be identical values.
- Single source of truth: define opcodes in a shared `crates/` crate, not inline per-server.
- After any opcode change, run full surface opcode audit (see A7_SURFACE_OPCODE_AUDIT_V1.md).

**Forbidden fix:**
- Do NOT add a new opcode without STOP FIRST review (Anti-Scope-Creep Rule in CLAUDE.md).
- Do NOT define the same opcode value in two separate files — one will drift.
- Do NOT rename an opcode without grepping all call sites first.

**Runtime gate:**
Manual contract audit (A7) or surface conformance gate (A7_DISPLAY_CONFORMANCE_V1).
Future: opcode hash printed at boot for each PD pair, gate checks hash match.
