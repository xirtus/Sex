This is now at a point where your system is no longer “designing an OS” — it’s doing something more important:

> you’ve stabilized a **formal execution contract layer over an experimental kernel**

So I’ll tighten what you have into something that is actually *machine-orchestrator safe* (Claude/Gemini won’t drift, won’t re-interpret, won’t re-plan).

Right now your plan is good, but one thing is still slightly too loose:

> ❌ “team applies diffs” language
> ❌ embedded reasoning inside runner script
> ❌ spec + execution mixed together

We fix that by making it **3-layer deterministic protocol**:

---

# 🧱 SEXOS PHASE 25.1 — FINAL ORCHESTRATION PROTOCOL (NO DRIFT VERSION)

## 🧠 LAYER 0 — IMMUTABLE SPEC (READ ONLY)

* `IPCPKU_MAP.md`
* `docs/phase25-compositor.md`

These are **not referenced for interpretation**, only for string-matching.

---

## ⚙️ LAYER 1 — EXECUTION ENGINE (ONLY THING THAT RUNS)

### ✔️ Replace your runner with this stricter form:

```bash
#!/bin/bash
set -euo pipefail

echo "[SEXOS PHASE 25.1] EXECUTION ENGINE START"

SPEC_A="/home/workdir/attachments/IPCPKU_MAP.md"
SPEC_B="docs/phase25-compositor.md"

test -f "$SPEC_A"
test -f "$SPEC_B"

echo "[STEP 1] CLAUDE KERNEL PATCH"
claude code run < prompts/claude_kernel_patch.txt

echo "[STEP 2] CLAUDE USERSPACE PATCH"
claude code run < prompts/claude_userspace_patch.txt

echo "[STEP 3] GEMINI VALIDATION"
gemini run < prompts/gemini_validate.txt

echo "[STEP 4] BUILD"
./build_payload.sh && make iso

echo "[STEP 5] RUN"
make run-sasos

echo "[SEXOS] PHASE 25.1 COMPLETE"
```

---

## 🧾 LAYER 2 — AGENT PROMPTS (NO FILE ACCESS, NO CONTEXT EXPANSION)

### 🔴 Claude Kernel Patch Prompt (`prompts/claude_kernel_patch.txt`)

```text
STRICT PATCH MODE — NO DESIGN AUTHORITY

You are ONLY allowed to:
- apply diffs
- remove code
- insert code exactly as specified

DO NOT:
- read files
- redesign architecture
- explain anything
- change structure beyond syscall 28 + init capability wiring

TASKS:
1. kernel/src/syscalls/mod.rs:
   - DELETE ALL r9 usage
   - REMOVE ANY PdxListenResult references
   - KEEP ONLY register ABI (rax,rsi,rdx,r10,r8)

2. kernel/src/init.rs:
   - ensure SLOT_DISPLAY + SLOT_SHELL capability grants exist

OUTPUT: unified diff only
```

---

### 🔵 Claude Userspace Patch Prompt (`prompts/claude_userspace_patch.txt`)

```text
STRICT PATCH MODE — USERSPACE ONLY

DO NOT:
- interpret IPC design
- read unrelated files
- modify kernel code
- introduce new abstractions

TASK:
1. crates/sex-pdx/src/lib.rs:
   - remove all legacy IPC structs
   - keep ONLY:
     PdxMessage
     SLOT_* constants
     OP_* constants
     pdx_listen (register syscall ABI only)

2. servers/sexdisplay/src/main.rs:
   - enforce:
     - idle frame (non-black)
     - error frame (red strip)
     - opcode handling: 0xDE, 0xDF, 0x101, 0xDD

OUTPUT: unified diff only
```

---

### 🟢 Gemini Validator Prompt (`prompts/gemini_validate.txt`)

```text
READ ONLY VALIDATION

DO NOT MODIFY CODE

CHECK:
1. No PdxListenResult exists anywhere
2. syscall 28 uses ONLY registers:
   rax, rsi, rdx, r10, r8
3. sexdisplay has:
   - idle frame path
   - error frame path
4. SLOT_DISPLAY and SLOT_SHELL exist and match IPCPKU_MAP

OUTPUT:
PASS or FAIL ONLY
If FAIL: list file + line references
```

---

## 🔒 LAYER 3 — WHAT YOU JUST ACHIEVED (IMPORTANT)

This structure fixes your original systemic issue:

### BEFORE

* agents “understood” system
* agents reinterpreted IPC model
* ABI drift reintroduced via creativity
* spec kept being rewritten implicitly

---

### AFTER (THIS DESIGN)

You now have:

| Layer  | Function               |
| ------ | ---------------------- |
| Spec   | immutable truth        |
| Runner | deterministic sequence |
| Claude | diff generator only    |
| Gemini | static verifier only   |

No agent can:

* redesign IPC
* reinterpret MPK model
* introduce r9 / struct ABI again
* “improve architecture”

---

## 🧠 KEY INSIGHT (WHY THIS STOPS YOUR BUG CLASS)

Your previous black screen / ABI issues were NOT kernel bugs.

They were:

> **semantic drift between agents acting as architects instead of patchers**

Now:

* Claude = compiler patch tool
* Gemini = lint tool
* Shell = execution harness

That’s it.

---

## 🚀 OPTIONAL NEXT STEP (IF YOU WANT TO GO HARDER)

I can upgrade this into a:

### “SEXOS CI KERNEL PIPELINE”

* git hook enforces IPCPKU_MAP compliance
* syscall ABI static checker
* framebuffer “non-black proof test”
* MPK simulation sandbox (userspace isolation validator)
* auto-revert on PDX drift

Just say **“harden CI layer”** and I’ll build it.
