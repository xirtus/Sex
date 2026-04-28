0. DESIGN PRINCIPLE (IMPORTANT)

The runner does NOT analyze, does NOT interpret, does NOT patch logic.

It only:

calls sex-debug
calls Gemini debugger agent
calls Claude for patch application
enforces ordering + failure gating

Everything else already exists.

1. TOOL ROLES (FINAL)
🧠 sex-debug (primary truth engine)
kernel state introspection
scheduler state
IPC ring inspection
framebuffer state
syscall traces

👉 THIS replaces most Gemini “truth prompts”

🧠 Gemini local debugger agent
cross-file static analysis
structural mismatch detection
capability graph validation

👉 ONLY used for inconsistencies

🔧 Claude CLI
ONLY applies diffs
NEVER reasons about system state
2. EXECUTION MODEL (CLEAN PIPELINE)
sex-debug (runtime truth)
    ↓
Gemini debugger (static validation)
    ↓
Claude (patch application)
    ↓
sex-debug (post-check)
    ↓
Gemini (final consistency check)
3. SINGLE RUNNER SCRIPT (REAL VERSION)
File: sexos_phase25_runner.sh
#!/usr/bin/env bash
set -euo pipefail

echo "[SEXOS] Phase 25 Orchestrator (sex-debug native)"

########################################
# PHASE 0 — RUNTIME TRUTH (sex-debug)
########################################

echo "[PHASE 0] sex-debug runtime snapshot..."

sex-debug dump > /tmp/runtime_snapshot.json

sex-debug ipc > /tmp/ipc_state.json

sex-debug scheduler > /tmp/sched_state.json

sex-debug framebuffer > /tmp/fb_state.json


########################################
# PHASE 1 — STATIC VALIDATION (Gemini agent)
########################################

echo "[PHASE 1] Gemini static validation..."

gemini-debugger analyze <<EOF > /tmp/static_report.json
Input:
- runtime_snapshot.json
- ipc_state.json
- sched_state.json

Check:
- PdxMessage correctness (no register semantics)
- Option<PdxMessage> enforcement
- SLOT_DISPLAY / SLOT_SHELL correctness
- sexdisplay render-per-tick invariant
- opcode coverage completeness
EOF


########################################
# PHASE 2 — PATCH GENERATION (Claude ONLY)
########################################

echo "[PHASE 2] Claude patching..."

claude run <<EOF > /tmp/patch.diff
Fix ONLY issues identified in /tmp/static_report.json.

Constraints:
- DO NOT modify kernel scheduler unless explicitly required
- DO NOT introduce register-based IPC logic
- MUST preserve Option<PdxMessage> semantics
- MUST enforce framebuffer-per-tick invariant in sexdisplay

Return unified diff only.
EOF


########################################
# APPLY PATCH
########################################

apply_patch /tmp/patch.diff


########################################
# PHASE 3 — POST-TRUTH CHECK (sex-debug)
########################################

echo "[PHASE 3] post-patch verification..."

sex-debug framebuffer > /tmp/fb_post.json

sex-debug ipc > /tmp/ipc_post.json

sex-debug scheduler > /tmp/sched_post.json


########################################
# PHASE 4 — FINAL CONSISTENCY CHECK (Gemini)

echo "[PHASE 4] final validation..."

gemini-debugger analyze <<EOF > /tmp/final_report.json
Input:
- fb_post.json
- ipc_post.json
- sched_post.json

Verify:
- no black framebuffer state
- idle frame exists under IPC starvation
- no opcode drop paths
- SLOT mapping consistency
- no sentinel-based IPC logic anywhere
EOF


########################################
# RESULT
########################################

echo "[SEXOS] Phase 25 complete."

cat /tmp/static_report.json
cat /tmp/final_report.json
