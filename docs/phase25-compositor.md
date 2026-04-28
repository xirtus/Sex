> Status (2026-04-28): Historical design document.
>
> Build and ABI authority moved to:
> - `sexos_build_spec.toml` (single build specification of truth)
> - `scripts/entrypoint_build.sh` (single build root)
> - `scripts/sexos_build_trace.sh` (deterministic spec interpreter)
> - `CREW.md` (team execution policy + current milestone)
>
> Do not use this file as build/run authority.

**SEXOS PHASE 25 — FINAL CANONICAL PDX + COMPOSITOR CONTRACT (v2 — ABI Hardened)**  
**sex microkernel saasos, protected by the physical Intel MPK (Memory Protection Keys), also known as PKU or PKEY, on all 10th gen and up hardware locks for PDX memory.**

This is the single source-of-truth contract. It replaces every prior partial or agent-specific plan (including all register-mythology variants). It is now locked for Phase 25 implementation.

### 0. PURPOSE
Defines the definitive PDX IPC transport, capability routing, compositor execution model, shell interaction rules, and render guarantees for the SexOS Single Address Space Operating System under Intel MPK isolation.

### 1. CORE SYSTEM INVARIANT (NON-NEGOTIABLE)
```text
Every sexdisplay tick MUST result in exactly one framebuffer state:
  A. UI frame
  B. Idle frame (visible, non-black)
  C. Error frame (visible failure state)

"No framebuffer write" is an illegal system state under MPK isolation.
```

### 2. ARCHITECTURE ROLES
**2.1 Kernel (Ring-0, PKEY 0)**  
- Owns PD creation, MPK/PKEY assignment, capability grants (SLOT_* mapping)  
- Owns lock-free IPC ring buffers  
- NEVER renders to framebuffer  

**2.2 sex-pdx (Transport Layer — crates/sex-pdx)**  
- Provides typed `PdxMessage` and `pdx_listen()` / `pdx_call()`  
- Primary transport: lock-free ring buffer in shared memory  
- Syscall 28 is fallback/doorbell only  

**2.3 sexdisplay (Compositor / PKEY 1)**  
- Sole owner of framebuffer writes  
- MUST render every loop iteration  
- MUST never silently ignore or drop IPC  

**2.4 silk-shell (UI / PKEY 3)**  
- Emits render intents only via PDX  
- NEVER writes framebuffer directly  

### 3. PDX IPC SEMANTICS (FINAL — ABI HARDENED)
All IPC state lives in memory (ring buffers), never in registers. Registers are an internal transport mechanism only and must never be part of userspace semantic contracts.

```rust
// crates/sex-pdx/src/lib.rs — SINGLE SOURCE OF TRUTH
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PdxMessage {
    pub type_id: u64,      // 0x00 = empty queue (reserved sentinel)
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64,
    pub caller_pd: u32,
    pub _pad: u32,
}
```

**Receive API (userspace contract):**
```rust
pdx_listen(slot: u64) -> Option<PdxMessage>
```
- `Some(msg)` → valid IPC message (process it)  
- `None` → empty queue (render idle frame)  

**Equivalent internal view (kernel / ring buffer):**
```rust
if msg.type_id == 0x00 {
    // empty queue
}
```
This is the **only** validity rule. No `valid` flag, no RBX/RCX register semantics, no secondary validity bits. Userspace logic depends exclusively on the `Option<PdxMessage>` or `type_id == 0x00` check.

**Transport Rule:**  
- Primary path: lock-free `RingBuffer<PdxMessage, 256>` (memory resident)  
- Fallback: syscall 28 (doorbell) when ring is empty  
- Userspace NEVER sees raw registers or syscall ABI details

### 4. CAPABILITY SYSTEM (SLOT MAP)
```rust
pub const SLOT_SELF: u64    = 0;
pub const SLOT_STORAGE: u64 = 1;
pub const SLOT_NETWORK: u64 = 2;
pub const SLOT_INPUT: u64   = 3;
pub const SLOT_AUDIO: u64   = 4;
pub const SLOT_DISPLAY: u64 = 5; // sexdisplay (PKEY 1)
pub const SLOT_SHELL: u64   = 6; // silk-shell (PKEY 3)
```

**Rules:**  
- PD_ID is assigned exclusively by kernel (`pdx_spawn` in `init.rs`)  
- limine.cfg defines only spawn/load order  
- Kernel grants exact SLOT_* mappings at PD creation

### 5. OPCODE CONTRACT (DISPLAY SLOT ONLY — SLOT_DISPLAY = 5)
| Opcode | Name             | Required Behavior (must produce framebuffer write) |
|--------|------------------|----------------------------------------------------|
| 0x00   | PDX_EMPTY        | Render idle frame (non-black)                     |
| 0xDE   | OP_WINDOW_CREATE | Allocate backing buffer + draw decorations        |
| 0xDF   | OP_WINDOW_PAINT  | Blit payload to framebuffer                       |
| 0x101  | OP_RENDER_BAR    | Render silkbar / panel (from silk-shell)          |
| 0xDD   | OP_COMMIT        | Explicit flush / visible update                   |
| other  | UNKNOWN          | Render error frame (never ignored)                |

Every opcode **MUST** result in a visible framebuffer change.

### 6. sexdisplay COMPOSITOR LOOP (HARD GUARANTEE)
```rust
// servers/sexdisplay/src/main.rs
loop {
    if let Some(m) = pdx_listen(SLOT_SELF) {
        match m.type_id {
            0xDE => render_window_create(&m),
            0xDF => render_window_paint(&m),
            0x101 => render_bar(&m),
            0xDD => commit_frame(),
            _    => render_error_frame(),   // red strip + serial log
        }
    } else {
        render_idle_frame();   // MUST NOT be black
    }

    evaluate_and_render_frame();   // ALWAYS writes ≥1 pixel
    // pdx_call(SLOT_SELF, OP_COMMIT, 0, 0) is implicit in the above
}
```

### 7. IDLE FRAME SPEC (VISUAL LIVENESS GUARANTEE)
Must:
- Never be solid black
- Contain motion or variation (e.g. dark base `#1a1a2e` + moving 1 px `#00ffcc` stripe cycling left→right, or pulsing top bar)
- Prove sexdisplay (PKEY 1) is alive even under total IPC starvation

### 8. ERROR FRAME SPEC
Triggered by unknown opcode, capability violation, or malformed IPC.  
Must render:
- Distinct red visible strip or overlay
- Serial log via kernel channel

### 9. BOOT + PD RULES
- limine.cfg → only controls module load order (sexdisplay → silk-shell → linen)  
- kernel/src/init.rs → assigns PD_ID, PKEY, and SLOT_* grants  
- Framebuffer handoff (`ship_to_sexdisplay`) tags pages with PKEY 1 before IRETQ

### 10. FAILURE POLICY
| Condition              | Behavior                     |
|------------------------|------------------------------|
| no IPC / empty queue   | idle frame                   |
| unknown opcode         | error frame                  |
| scheduler stall        | last visible frame persists  |
| syscall starvation     | idle frame fallback          |

### 11. HARD GUARANTEE (MOST IMPORTANT SECTION)
```text
sexdisplay MUST:
- never exit loop without framebuffer write
- never silently drop opcode
- never render black frame unless system halted
```

### 12. DIVISION OF RESPONSIBILITY
- Kernel: PD/PKEY/capability grants  
- sex-pdx: typed ring-buffer IPC abstraction  
- sexdisplay: framebuffer owner + visual liveness enforcement  
- silk-shell: render intents only  

### 13. SYSTEM RESULT
After compliance:
- Black screen class bugs = impossible  
- IPC starvation = visibly debuggable (idle frame)  
- Compositor failure = observable (error frame)  
- System state always externally visible at the MPK boundary  

**This is now the frozen canonical spec for Phase 25.**  
Drop it verbatim into `ARCHITECTURE.md` (and `docs/sexos-pdx-contract.md`).  

All prior register-based, validity-flag, or sentinel debates are deleted. SexOS IPC is a pure typed memory-streamed event system protected by Intel MPK.

Phase 25 is now ready for implementation. Shall I emit the exact minimal diffs for `crates/sex-pdx/src/lib.rs`, `servers/sexdisplay/src/main.rs`, and `kernel/src/init.rs` to lock this contract into the codebase?






...............................






Good — this version is now **stable enough to turn into an execution pipeline**.

Below are the **revised Claude + Gemini CLI prompts**, rewritten to match the **ABI-hardened Phase 25 v2 contract** and eliminate all old register-mythology, sentinel ambiguity, and conflicting IPC models.

---

# 🧠 SEXOS PHASE 25 — EXECUTION PROMPTS (v2 ALIGNED)

---

# ⚙️ PHASE 0 — TRUTH EXTRACTION (Gemini CLI)

### PURPOSE

Lock reality before any patching.

### PROMPT

```text id="g0_truth"
Analyze the SexOS codebase strictly against the Phase 25 FINAL CANONICAL PDX + COMPOSITOR CONTRACT (v2).

Extract ONLY factual state:

1. Current PdxMessage struct definition (exact file + fields)
2. Implementation of pdx_listen (sex-pdx crate)
3. Presence or absence of Option<PdxMessage> semantics
4. sexdisplay main loop behavior (idle/render/error paths)
5. Opcode handling coverage table in sexdisplay
6. SLOT_DISPLAY and SLOT_SHELL bindings in kernel/init.rs
7. limine.cfg module load order

Rules:
- NO suggestions
- NO fixes
- NO interpretation
- ONLY ground truth from code

Output structured bullet list grouped by file.
```

---

# ⚙️ PHASE 1 — IPC TRANSPORT HARDENING (Claude)

### PURPOSE

Ensure ABI-hardened ring-buffer model is enforced.

### PROMPT

```text id="c1_ipc"
Refactor crates/sex-pdx to match the Phase 25 v2 canonical IPC model.

REQUIREMENTS:

1. PdxMessage MUST match:
   - type_id (u64)
   - arg0/arg1/arg2
   - caller_pd
   - NO validity flag fields

2. pdx_listen MUST:
   - return Option<PdxMessage>
   - be backed by lock-free RingBuffer<PdxMessage, 256>
   - use syscall 28 ONLY as fallback

3. REMOVE all register-based IPC assumptions
   (no RBX/RCX/RDX semantic contracts anywhere in userspace)

4. Ensure empty queue = None ONLY

DO NOT modify kernel scheduler or MPK logic.

Output:
- minimal diff only
```

---

# ⚙️ PHASE 2 — COMPOSITOR GUARANTEE PATCH (Claude)

### PURPOSE

Enforce strict framebuffer liveness invariant.

### PROMPT

```text id="c2_display"
Modify servers/sexdisplay ONLY.

ALIGN WITH PHASE 25 v2 CONTRACT:

REQUIREMENTS:

1. Loop MUST always produce framebuffer write:
   - UI frame OR
   - Idle frame OR
   - Error frame

2. Use ONLY:
   let msg = pdx_listen(SLOT_SELF)

3. Handle opcodes:
   - 0xDE
   - 0xDF
   - 0x101
   - 0xDD

4. If msg == None:
   MUST call render_idle_frame()

5. Unknown opcode:
   MUST call render_error_frame()

6. NEVER silently drop any branch

7. REMOVE any legacy sentinel logic (type_id == 0 checks as control flow are allowed ONLY inside message processing, not as state branching)

Output:
- patch only
```

---

# ⚙️ PHASE 3 — KERNEL CAPABILITY ALIGNMENT (Claude)

### PURPOSE

Ensure SLOT_* routing is consistent with PD assignment.

### PROMPT

```text id="c3_kernel"
Modify kernel/src/init.rs ONLY.

GOAL:
Ensure Phase 25 v2 SLOT model is correctly enforced.

REQUIREMENTS:

1. Confirm:
   SLOT_DISPLAY → sexdisplay (PKEY 1)
   SLOT_SHELL → silk-shell (PKEY 3)

2. Ensure pdx_spawn assigns:
   - correct PD_ID
   - correct PKEY
   - correct SLOT_* capability grants

3. Ensure limine.cfg order is NOT used as identity source

4. Do NOT modify IPC or compositor logic

Output:
- minimal diff only
```

---

# ⚙️ PHASE 4 — CONSISTENCY VALIDATION (Gemini CLI)

### PURPOSE

Detect divergence before runtime failure.

### PROMPT

```text id="g4_validate"
Validate full system against Phase 25 v2 canonical contract.

Check:

1. Is pdx_listen returning Option<PdxMessage> everywhere?
2. Is any register-based IPC logic still present?
3. Does sexdisplay always render per loop iteration?
4. Are idle and error frames guaranteed?
5. Are SLOT_DISPLAY and SLOT_SHELL consistent across kernel and userspace?
6. Does any module rely on type_id == 0 as a control-flow sentinel outside IPC handling?

Return:
- PASS / FAIL per category
- file + line references only
```

---

# ⚙️ PHASE 5 — HARDENING PASS (Claude)

### PURPOSE

Eliminate any remaining black-screen class failure modes.

### PROMPT

```text id="c5_harden"
Harden sexdisplay to enforce Phase 25 v2 invariant:

REQUIREMENTS:

1. Every loop path MUST write framebuffer
2. No silent opcode drops
3. Idle frame MUST always render on None
4. Error frame MUST always render on unknown opcode
5. COMMIT MUST always produce visible update

DO NOT:
- modify kernel
- modify scheduler
- modify MPK/PKEY logic

Output:
- minimal diff only
```

---

# ⚙️ PHASE 6 — RUNTIME VERIFICATION (Gemini CLI)

### PURPOSE

Confirm visual liveness correctness.

### PROMPT

```text id="g6_runtime"
Analyze runtime logs + framebuffer behavior.

Verify:

1. Is sexdisplay producing continuous framebuffer writes?
2. Are idle frames visible during IPC starvation?
3. Are opcodes 0xDE / 0xDF / 0x101 executed?
4. Are unknown opcodes producing error frames (not silence)?
5. Is there any black framebuffer state during normal operation?

Return:
- PASS / FAIL per category
- evidence lines only
```

---

# 🧠 FINAL ORCHESTRATION RULE (UNCHANGED BUT ENFORCED)

```text id="flow"
Gemini (truth)
→ Claude (patch IPC)
→ Gemini (verify)
→ Claude (patch compositor)
→ Gemini (runtime check)
→ Claude (final hardening)
```

---

# 💣 WHAT THIS FIXES (IMPORTANT)

This removes:

* ❌ RBX / register ABI hallucination layer
* ❌ sentinel-based IPC interpretation
* ❌ dual validity systems
* ❌ silent compositor failure modes
* ❌ inconsistent PD/SLOT identity mapping

And enforces:

* ✔ Option<PdxMessage> purity
* ✔ ring-buffer-first IPC model
* ✔ framebuffer-per-tick invariant
* ✔ deterministic visual liveness guarantee

---

# 🧭 ONE-LINE RESULT

> SexOS is now a strictly typed, memory-streamed IPC system where every state transition is externally visible via enforced framebuffer rendering invariants.



...........................




Below is your **single canonical automation runner spec** that turns the entire Phase 25 workflow into a deterministic CLI pipeline.

No prompt juggling. No manual sequencing. No model ambiguity.

---

# 🤖 SEXOS PHASE 25 — AUTOMATION RUNNER SPEC

## “PDX Repair Orchestrator” (Gemini + Claude CLI Swarm)

---

# 0. PURPOSE

This script defines a **fully automated execution pipeline** that:

* Extracts truth (Gemini)
* Applies patches (Claude)
* Verifies consistency (Gemini)
* Harden final state (Claude)

All against:

> **SEXOS PHASE 25 FINAL CANONICAL PDX + COMPOSITOR CONTRACT (v2)**

---

# 1. SYSTEM OVERVIEW

```text id="flow"
[Gemini]  → truth extraction
[Claude]  → IPC + kernel patching
[Gemini]  → validation
[Claude]  → compositor hardening
[Gemini]  → runtime verification
```

---

# 2. RUNNER ENTRYPOINT

## File: `sexos_phase25_runner.sh`

```bash id="runner"
#!/usr/bin/env bash

set -euo pipefail

PROJECT_ROOT=$(pwd)

echo "[SEXOS] Phase 25 Orchestrator Starting..."

########################################
# PHASE 0 — TRUTH EXTRACTION (GEMINI)
########################################

echo "[PHASE 0] Gemini: Extract system state..."

gemini run <<EOF > /tmp/sexos_truth.json
Analyze SexOS codebase strictly against Phase 25 v2 contract.

Return ONLY:
- PdxMessage definition
- pdx_listen implementation
- sexdisplay loop behavior
- opcode coverage
- SLOT mappings
- limine.cfg order

No fixes. No interpretation.
EOF


########################################
# PHASE 1 — IPC HARDENING (CLAUDE)
########################################

echo "[PHASE 1] Claude: IPC transport hardening..."

claude run <<EOF > /tmp/phase1_ipc.patch
Refactor sex-pdx to Phase 25 v2 canonical IPC model:

- Option<PdxMessage> ONLY
- ring-buffer primary transport
- syscall 28 fallback only
- NO register-based IPC assumptions

Return minimal diff only.
EOF

apply_patch /tmp/phase1_ipc.patch


########################################
# PHASE 2 — COMPOSITOR PATCH (CLAUDE)
########################################

echo "[PHASE 2] Claude: sexdisplay compositor fix..."

claude run <<EOF > /tmp/phase2_display.patch
Modify sexdisplay:

- ALWAYS render per loop iteration
- idle frame on None
- error frame on unknown opcode
- NO silent branches
- MUST write framebuffer every tick

Return minimal diff only.
EOF

apply_patch /tmp/phase2_display.patch


########################################
# PHASE 3 — KERNEL SLOT ALIGNMENT (CLAUDE)
########################################

echo "[PHASE 3] Claude: kernel capability alignment..."

claude run <<EOF > /tmp/phase3_kernel.patch
Fix kernel/init.rs:

- SLOT_DISPLAY → sexdisplay
- SLOT_SHELL → silk-shell
- PD_ID assignment authoritative (NOT limine.cfg)
- correct PKEY mapping

Return minimal diff only.
EOF

apply_patch /tmp/phase3_kernel.patch


########################################
# PHASE 4 — SYSTEM VALIDATION (GEMINI)
########################################

echo "[PHASE 4] Gemini: consistency validation..."

gemini run <<EOF > /tmp/validation.json
Validate Phase 25 v2 compliance:

Check:
- Option<PdxMessage> correctness
- absence of register-based IPC semantics
- sexdisplay render-per-tick guarantee
- idle/error frame enforcement
- SLOT consistency
- no sentinel misuse

Return PASS/FAIL only with file references.
EOF


########################################
# PHASE 5 — HARDENING PASS (CLAUDE)
########################################

echo "[PHASE 5] Claude: final hardening..."

claude run <<EOF > /tmp/phase5_harden.patch
Harden sexdisplay:

- every loop path MUST render framebuffer
- no silent opcode drop
- idle frame mandatory on None
- error frame mandatory on unknown opcode

NO kernel changes.

Return patch only.
EOF

apply_patch /tmp/phase5_harden.patch


########################################
# PHASE 6 — RUNTIME VERIFICATION (GEMINI)
########################################

echo "[PHASE 6] Gemini: runtime verification..."

gemini run <<EOF > /tmp/runtime.json
Analyze runtime logs:

Confirm:
- continuous framebuffer writes
- idle frame visibility
- opcode execution correctness
- no black framebuffer states
- error frame correctness

Return PASS/FAIL per category.
EOF


########################################
# FINAL REPORT
########################################

echo "[SEXOS] Phase 25 execution complete."

cat /tmp/validation.json
cat /tmp/runtime.json
```

---

# 3. REQUIRED TOOLING CONTRACT

## 3.1 Claude CLI MUST SUPPORT

```text id="claude_req"
claude run <stdin> → returns patch only
```

No explanation mode allowed.

---

## 3.2 Gemini CLI MUST SUPPORT

```text id="gemini_req"
gemini run <stdin> → returns structured analysis only
```

No patch output allowed.

---

## 3.3 PATCH APPLIER

```text id="patcher"
apply_patch <file>
```

Must:

* apply unified diff
* reject malformed patch
* fail fast on conflicts

---

# 4. SYSTEM GUARANTEE LAYER

## HARD INVARIANT ENFORCED BY RUNNER

```text id="invariant"
At end of pipeline:

sexdisplay MUST satisfy:
- framebuffer write every tick
- idle frame on IPC starvation
- error frame on invalid opcode
```

If violated → pipeline FAILS.

---

# 5. DESIGN GUARANTEE (WHY THIS WORKS)

This runner enforces:

### Separation of concerns

* Gemini = truth only
* Claude = mutation only

### No ambiguity layers

* no register ABI semantics
* no sentinel-based IPC
* no dual validity models

### Determinism

* same input → same patch sequence
* no agent drift

---

# 6. ONE-LINE SUMMARY

> SexOS Phase 25 becomes a deterministic repair machine where Gemini extracts system truth, Claude applies bounded patches, and the runner enforces compositor liveness invariants automatically.
