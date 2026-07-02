# Proof Discipline + Audit Gates — Audit Report

**Date:** 2026-05-03
**Context:** Premortem analysis before Bell/USB/Linen/Dock feature work
**Read first:** `docs/handoff/STABLE_BASELINE_20260503.md`, `CLAUDE.md`, `docs/SILK_DE_EXECUTION_PLAN.md`, `docs/INPUT_USB_NEXT.md`

---

## 1. Proof Discipline Coverage Report

### Currently Required (documented in STABLE_BASELINE.md)

| Proof Type | Where Required | Status |
|---|---|---|
| **boundary proof** | §5 "Hard Rule", §6 "Global Completion Rule" | ✅ Formalized |
| **negative proof** | §6 "Global Completion Rule" | ✅ Formalized |
| **integration proof** | §6 "INTEGRATED_SCENARIO_PROOF_V1" | ✅ Formalized |
| **handoff proof** | §5 "Recurring Bug Handoff Rule" | ✅ Formalized |
| **build proof** | §4 verification command (implicit) | ⚠️ Present but unnamed |
| **boot/runtime proof** | §4 grep markers (implicit) | ⚠️ Present but unnamed |
| **fault scan** | §4 ("NO [fault], [panic], [GP], [PF]") | ⚠️ Required implicitly, unnamed |
| **forbidden diff scan** | §5 anti-scope-creep validation cmd (example only) | ❌ Not formalized as a gate |

### Gap: Global Completion Rule is incomplete

STABLE_BASELINE §6 lists the required proofs as:
```
- boundary proof
- negative proof
- integration proof
- handoff proof
```

**Omitted:** build proof, boot/runtime proof, fault scan, forbidden diff scan, exact log markers.

### Consequence

A feature could pass all four stated proof requirements while silently:
- Editing kernel/ or crates/sex-pdx/ without authorization
- Writing framebuffer pixels from the shell or an app
- Spanning 3+ domains in a single patch
- Introducing POSIX/std/libc assumptions
- Redesigning shared-memory/backing-buffer patterns

This is exactly the "false-green completion" and "forbidden architecture drift" risk.

---

## 2. Audit Gate Coverage (Executable vs. Documented-Only)

### Existing Executable Gates

| Gate | Script | Scope |
|---|---|---|
| Forbid PdxListenResult | `scripts/sexos_pipeline.sh` | ABI register convention |
| Forbid r9 in IPC | `scripts/sexos_pipeline.sh` | ABI register convention |
| Forbid struct-pointer IPC | `scripts/sexos_pipeline.sh` | ABI register convention |
| Enforce register return contract | `scripts/sexos_pipeline.sh` | ABI register convention |
| Contract validation | `scripts/entrypoint_build.sh` | Build-time contract hashes |
| SilkBar contract validation | `scripts/entrypoint_build.sh` | silkbar-model gate functions |
| ABI hash check | `scripts/entrypoint_build.sh` | sex-pdx + syscalls hash |

### Missing Executable Gates

| Invariant | Documented In | Executable Gate? |
|---|---|---|
| No kernel edits without STOP FIRST | STABLE_BASELINE §2, CLAUDE.md | ❌ None |
| No sex-pdx edits without STOP FIRST | STABLE_BASELINE §2 | ❌ None |
| No framebuffer writes outside sexdisplay | STABLE_BASELINE §2, §5 | ❌ None |
| No input policy in sexdisplay | STABLE_BASELINE §5 boundary rules | ❌ None |
| No app lifecycle in sexdisplay | STABLE_BASELINE §5 boundary rules | ❌ None |
| No shell framebuffer writes | STABLE_BASELINE §2, §5 | ❌ None |
| No std/libc/thread/sleep/POSIX | CLAUDE.md, AGENT_README_FIRST.md | ❌ None |
| No shared-memory/backing-buffer redesign | STABLE_BASELINE §5, §8 | ❌ None |
| No broad multi-domain patch (≤2 domains) | STABLE_BASELINE §5 anti-scope-creep | ❌ None |

### Root Cause

Every invariant is **documented in markdown only**. The existing `sexos_pipeline.sh` only covers ABI register convention patterns. It does not touch domain boundary or architecture drift gates. No CI or pre-commit hook enforces any of the core architecture invariants.

---

## 3. Smallest Proposed Patch Plan

### Patch A — Docs Fix: Expand Global Completion Rule

**File:** `docs/handoff/STABLE_BASELINE_20260503.md`

Change §6 Global Completion Rule from:
```
Every feature must prove:
- boundary proof
- negative proof
- integration proof
- handoff proof
```

To:
```
Every feature must prove:
- boundary proof
- negative proof
- integration proof
- handoff proof
- build proof
- boot/runtime proof (exact log markers + fault scan pass)
- forbidden diff scan (git diff passes all invariant gates: no kernel edits,
  no sex-pdx edits, no framebuffer writes outside sexdisplay, ≤2 domains,
  no std/libc/thread, no backing-buffer redesign)
```

---

### Patch B — New Script: `scripts/audit_invariant_gates.sh`

**Purpose:** One small script (~120 lines) that checks all missing invariant gates against `git diff` (not full tree — only what changed).

**Logic for each gate:**

| # | Gate | Check |
|---|---|---|
| 1 | No kernel edits | `git diff --name-only | grep -c '^kernel/'` → fail if > 0 |
| 2 | No sex-pdx edits | `git diff --name-only | grep -c '^crates/sex-pdx/'` → fail if > 0 |
| 3 | No FB writes outside sexdisplay | `git diff -- '*.rs' | grep -E 'write.*fb|framebuffer'` ignoring sexdisplay/ |
| 4 | No shell pixel writes | `git diff -- 'servers/silk-shell/*.rs'` for pixel/FB patterns |
| 5 | No std/libc/thread imports | `git diff --name-only | grep '\.rs$' | xargs grep` for forbidden imports |
| 6 | ≤2 domains | Count changed dirs among: kernel, sex-pdx, sexdisplay, silk-shell, sexinput, sexusb, apps |
| 7 | No backing-buffer redesign | `git diff -- '*.rs'` for shared-buffer/backing-buffer keywords |

**Exit:** First failure exits with code 1 and a clear `[FAIL]` message.

---

### Patch C — Wire into pre-commit

**File:** `.githooks/pre-commit`

Add `./scripts/audit_invariant_gates.sh` after the existing pipeline call:
```bash
./scripts/sexos_pipeline.sh
./scripts/audit_invariant_gates.sh
```

---

## 4. Exact Files to Patch

| # | File | Type | Change |
|---|---|---|---|
| 1 | `docs/handoff/STABLE_BASELINE_20260503.md` | Edit | Expand Global Completion Rule with missing proof gates |
| 2 | `scripts/audit_invariant_gates.sh` | **New** | 7-gate invariant checker against git diff |
| 3 | `.githooks/pre-commit` | Edit | Add invariant gate call |

**Zero Rust source changes. Zero kernel edits. Zero sex-pdx edits. Zero server edits.**

---

## 5. STOP FIRST Conditions

Do NOT proceed with implementation if any of these are needed:
- Edits to `kernel/` source files
- Edits to `crates/sex-pdx/` source files
- Edits to any server source file (`servers/sexdisplay/`, `servers/silk-shell/`, etc.)
- New Rust crate dependencies
- ABI/opcode changes
- Renderer feature additions

The patches above are docs-only + shell-script-only + git-hook-only.

---

## 6. Validation Commands (after patches applied)

```bash
# Verify new script exists and is executable
ls -la scripts/audit_invariant_gates.sh

# Run against current dirty tree (should detect modified files)
./scripts/audit_invariant_gates.sh

# Verify pre-commit hook references both scripts
cat .githooks/pre-commit

# Verify Global Completion Rule updated
grep -n "forbidden diff scan\|build proof\|boot/runtime proof" \
  docs/handoff/STABLE_BASELINE_20260503.md

# Verify no Rust code changed
git diff -- '*.rs' | wc -l
# Expected: 0
```

---

## 7. Current Tree Status (at time of audit)

```
git status shows modified files in:
  CLAUDE.md, silkbar-model, docs/*.md, kernel/src/init.rs, kernel/src/lib.rs,
  servers/sexdisplay/, servers/sexinput/, servers/silk-shell/
  serial.log

Untracked:
  docs/AGENT_*, docs/COMMON_FAILURES.md, docs/PDX_QUICKMAP.md,
  docs/PD_MEMORY_CAPABILITY_MAP.md, docs/SILK_DE_EXECUTION_PLAN.md.bak,
  docs/SexOS_Storage_Architecture.pdf, an/, kernel/src/pd_diagnostic.rs,
  run_and_debug.sh, scripts/agent_preflight.sh, servers/silk-shell/src/main.rs.bak

Recent commits:
  a5fa26b fix(silkbar): use typed ChipSlot indices for M2 ABI
  a5e0af7 docs: add boundary and interaction hardening plans
  93de92d fix(kernel): make boot frame allocation cursor-based
  557dd45 refactor(shell): consolidate OS panel toggles
  4e52488 feat(silkbar): toggle status panel from status click
```

*End of audit report. Save this document before making any changes.*
