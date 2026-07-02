# K2 Namespace Audit and Design

**Date:** 2026-05-05
**Reviewer:** Real Claude (claude-sonnet-4-6)
**Scope:** J1-J7 namespace usage vs IPCPKU_MAP + rapid source documents
**Mode:** Read-only analysis. No code touched.

---

## Executive Verdict

```
SAFE_LOCAL_PATCH
```

No namespace violation crosses ABI, PDX slot, opcode, or PKEY boundaries.
All issues are shell-local. All needed constants/corrections are docs-only or
single-file local-constant edits in `servers/silk-shell/src/main.rs`.
No STOP FIRST triggered.

---

## IPCPKU_MAP Source Path

**Canonical:** `/home/xirtus_arch/Documents/microkernel/IPCPKU_MAP.md`
**Mirror:** `docs/IPCPKU_MAP.md` (must not diverge from canonical)

---

## Namespace Source-of-Truth Summary

### IPCPKU_MAP defines:

| Namespace | Range | Owner |
|-----------|-------|-------|
| PKEY (MPK protection keys) | 0=Kernel, 1=sexdisplay, 2=sext, 3=silk-shell, 4+=dynamic | Hardware/kernel |
| PDX slots | 1=STORAGE, 2=SEXT, 3=INPUT, 4=AUDIO, 5=DISPLAY, 6=SHELL, 7=SILKBAR, 8=USB_HOST, 9=USB_SEXINPUT(kernel-local), 10=SEXSTORE, 11=QUIL | sex-pdx crate |

**IPCPKU_MAP does NOT define:** surface IDs, object IDs, buffer IDs, scancode assignments, grant_ref values, Bell/Collar/Mesh local enum values. These are all shell-local namespaces that have no canonical document.

### sex-pdx crate defines (opcodes):

| Opcode | Constant | Direction |
|--------|---------|-----------|
| 0x14 | OP_SHELL_BIND_BUFFER | shell→display |
| 0x15 | OP_DISPLAY_SET_SNAPSHOT | shell→display |
| 0xB0 | OP_KV_GET | shell→sexstore |
| 0xB1 | OP_KV_PUT | shell→sexstore |
| 0xD0 | OP_QUIL_PING | shell→quil |
| 0xEB | OP_SURFACE_UPDATE | shell→display |
| 0xEC | *(unnamed)* | shell→display: create/move surface |
| 0xEE | OP_SURFACE_DEACTIVATE | shell→display |
| 0xEF | *(unnamed)* | shell→display: fill rect |
| 0xF0-0xF4 | OP_SILKBAR_PING/GET_ABI/UPDATE/WORKSPACE_ACTIVE/FOCUS_STATE | shell→silkbar |
| 0xFB | OP_SCENE_SETTINGS_CMD | shell→display |
| 0xFC | OP_APPEARANCE_TOKENS | shell→display |
| 0xFD | OP_SURFACE_TAB_INFO | shell→display |
| 0x202 | OP_HID_EVENT | sexinput→shell |
| 0x260 | OP_USB_MOUSE_REPORT | sexusb→shell |

### Surface IDs (shell-local, shared with sexdisplay at call time):

| Range | IDs | Tier |
|-------|-----|------|
| 0x90-0x97 (144-151) | CURSOR, -, LAUNCHER, STATUS, CLOCK, BELL, SCENE_SETTINGS, ATLAS | OS UI panels |
| 100-103 | APP, STATIC, TEST3, TEST4 | App surfaces |
| 200-204 | LINEN, QUIL, MESH, COLLAR, BELL_PLACEHOLDER | Workstation surfaces |

---

## Violation Table

| # | Severity | Location | Violation | Notes |
|---|----------|----------|-----------|-------|
| V1 | MEDIUM | (no file) | No canonical document specifies shell-local namespace tiers | IPCPKU_MAP only covers PKEYs and PDX slots. Surface IDs, object IDs, buffer IDs are unspecified anywhere. |
| V2 | LOW | main.rs:65,95 | Two "Bell" surfaces with ambiguous names | `SURFACE_ID_BELL = 0x95` (OS panel) vs `SURFACE_ID_BELL_PLACEHOLDER = 204` (workstation stub). "Bell" prefix reused across different tiers. |
| V3 | LOW | main.rs:1597 | Comment "PrintScreen — J4 test trigger" misleads; 0x59 is not standard PS/2 PrintScreen | In PS/2 Set 1, 0x59 = NumPad 1 (End) in some layouts. Real PrintScreen is multi-byte (E0 2A E0 37). Comment is accurate about intent ("test trigger") but wrong about key identity. |
| V4 | LOW | main.rs:214,466 | `grant_ref: u64` field has no documented meaning for stub/placeholder range | All seeds use 0. Dynamic J4 buffers copy object's grant_ref (also 0). No constant `GRANT_REF_STUB = 0` exists to make the placeholder intent explicit. |
| V5 | LOW (residual K2C) | main.rs:495,517 | Seed buffer linen_object_refs 2 and 5 set without J4 proof trail | Ghost links emit from Mesh at boot with no Collar/Bell trace. Not a namespace violation but a data coherence gap. See K2C. |
| V6 | RESOLVED (K2A) | main.rs:437 | ~~J4 dynamic buffer_id = object_id collided with seed buffer IDs~~ | Fixed: `QUIL_DYNAMIC_BUFFER_ID_BASE = 1000`. Dynamic IDs = 1001-1016. No overlap with seeds (1-6) or object IDs (1-16). |

---

## Corrected Namespace Table

All namespaces are shell-local (PKEY 3 domain) unless noted.

### PDX/ABI (IPCPKU_MAP canonical — do not touch without STOP FIRST):

| Namespace | Range | Doc |
|-----------|-------|-----|
| PKEY | 0-3 assigned, 4+ dynamic | IPCPKU_MAP.md |
| PDX slots | 1-11 assigned | IPCPKU_MAP.md + sex-pdx/src/lib.rs |
| Opcodes | 0x14-0x260 (sparse) | sex-pdx/src/lib.rs |

### Shell-local (silk-shell only, no ABI contract):

| Namespace | Range | Current | Correct | Document |
|-----------|-------|---------|---------|---------|
| Linen object IDs | 1-16 | 1-6 seeds used | LINEN_OBJECT_ID_MIN=1, LINEN_OBJECT_ID_MAX=LINEN_MAX_OBJECTS | needs K2B doc |
| Quil seed buffer IDs | 1-16 | 1-6 seeds used | QUIL_SEED_BUFFER_ID_MAX=6 (implies ≤LINEN_MAX_OBJECTS but distinct type) | needs K2B doc |
| Quil dynamic buffer IDs | 1001-1016 | 1000+object_id (K2A) | Correct as-is | K2A_FIX_QUIL_BUFFER_ID_COLLISION_V1.md |
| Surface IDs — app | 100-103 | 100-103 | No change | main.rs constants |
| Surface IDs — workstation | 200-204 | 200-204 | Rename 204 to `SURFACE_ID_BELL_WORKSTATION` for clarity | K2D |
| Surface IDs — OS panels | 0x90-0x97 | 0x90-0x97 | No change | main.rs constants |
| grant_ref placeholder | 0 | all zeros | Add `GRANT_REF_STUB: u64 = 0` constant | K2D |
| CollarOperation | 0-6 | 0-6 | No change (shell-local enum) | — |
| CollarDecision | 0-4 | 0-4 | No change (shell-local enum) | — |
| BellEventKind | 0-3 | 0-3 | No change (shell-local enum) | — |

### Future reserved ranges (not yet implemented):

| Namespace | Reserved | Rationale |
|-----------|----------|-----------|
| Linen object IDs | 17-999 | Gap before dynamic buffer base |
| Quil seed buffer IDs | 7-999 | Gap before dynamic buffer base |
| Quil dynamic buffer IDs | 1000-1999 | 1000+object_id (current rule) |
| Surface IDs | 205-0x8F | Unassigned workstation expansion room |
| Surface IDs | 104-143 | Unassigned app expansion room |
| PDX slots | 12+ | Future servers (reserved by IPCPKU_MAP process) |
| Opcodes | 0xFE-0x1FF | Unassigned; do NOT use without STOP FIRST |

---

## K2 Patch Sequence (smallest first)

### K2B — Namespace spec doc (docs only, no code)
**What:** Write `docs/handoff/K2B_SHELL_LOCAL_NAMESPACE_SPEC_V1.md` enumerating all shell-local ID tiers with explicit ranges, rules, and no-overlap guarantees.
**Files:** docs only.
**Safe for deepseekclaude:** Yes (docs-only, no logic).

### K2C — Seed data coherence (code: seed array only)
**What:** Fix ghost links in seed data. Options:
- Option A (clean): Remove `linen_object_ref` from seed buffers that have no J4 proof trail. Set buffer 2 ref=0 and buffer 4 ref=0. Removes ghost Mesh rows at boot.
- Option B (explicit init): Add a boot-time seed-link pass that calls a lightweight version of step 5 (update LinenObject.linked_surface_id) for pre-seeded refs. Adds coherence without removing the pre-links.

Recommendation: **Option A**. Seeds should be clean stubs. Real object links flow via J4 only.

**Files:** `servers/silk-shell/src/main.rs` (QUIL_SEED_BUFFERS const only).
**Requires real Claude:** Yes — touching seed data semantics. Do NOT delegate to deepseekclaude.

### K2D — Minor constant/comment cleanup (code: small constants only)
**What:**
1. Add `const GRANT_REF_STUB: u64 = 0;` with comment "placeholder: no real Collar grant".
2. Fix comment on 0x59 from "PrintScreen" to "test trigger (not standard PS/2 key)".
3. Optionally rename `SURFACE_ID_BELL_PLACEHOLDER` → `SURFACE_ID_BELL_WORKSTATION` for clarity.

**Files:** `servers/silk-shell/src/main.rs` (constants + one comment).
**Safe for deepseekclaude:** Items 1 and 2 only. Item 3 requires rename across all usages — real Claude.

### K2E — Namespace rules in IPCPKU_MAP (docs only)
**What:** Add a "Shell-Local Namespaces" section to `IPCPKU_MAP.md` cross-referencing K2B spec.
**Files:** `IPCPKU_MAP.md` only.
**Safe for deepseekclaude:** Yes (docs-only, additive).

---

## Do Not Fix Yet

| Item | Reason |
|------|--------|
| Quil dynamic buffer ID base (K2A result) | Already fixed. Do not revisit. |
| Surface ID rename (SURFACE_ID_BELL_PLACEHOLDER) | Rename ripples across all usages. Low priority. Real Claude if pursued. |
| Grant ref namespace beyond 0 | Not real until Collar is real. STOP FIRST. |
| New PDX opcodes for multi-rect display | STOP FIRST: ABI edit. |
| Linen/Quil PD migration | STOP FIRST: new PDX ops + cross-PD object handoff protocol. |

---

## What Requires STOP FIRST

| Item | Why |
|------|-----|
| Any new opcode (0xEC unnamed, 0xEF unnamed → name only without new op = OK) | sex-pdx ABI edit |
| Adding new PDX slots beyond 11 | IPCPKU_MAP edit + kernel route table |
| Changing PKEY assignments | kernel/MPK config |
| Real Collar grant_ref semantics | New Collar PD + PDX ABI |
| Moving object/buffer tables out of silk-shell | New PDX ops + capability grants |

---

## First deepseekclaude Prompt to Run

**K2B namespace spec doc (safest, docs-only):**
```
Write docs/handoff/K2B_SHELL_LOCAL_NAMESPACE_SPEC_V1.md.

Content: formal namespace spec for all shell-local ID tiers in silk-shell.
Use table from docs/handoff/K2_NAMESPACE_AUDIT_DESIGN_V1.md §Corrected Namespace Table as source.
Include:
- shell-local object IDs (1-16, LINEN_MAX_OBJECTS)
- shell-local seed buffer IDs (1-6)
- dynamic buffer IDs (1000+object_id, QUIL_DYNAMIC_BUFFER_ID_BASE=1000)
- surface ID tiers (OS panels 0x90-0x97, app 100-103, workstation 200-204)
- future reserved ranges
- explicit rule: dynamic buffer IDs must not collide with seed IDs or surface IDs
- explicit rule: all these namespaces are shell-local; they do NOT appear in PDX opcodes or ABI
No code. No edits to main.rs. Docs only.
Commit: docs(namespace): add shell-local ID namespace spec
```

---

## What Requires Real Claude Next

- **K2C seed data coherence** — requires judgment on Option A vs B + understanding seed intent
- **SURFACE_ID_BELL_PLACEHOLDER rename** if pursued — multi-site rename with semantic implications
- Any future addition of new named opcode constants for currently-unnamed 0xEC/0xEF
- Any decision to promote shell-local object/buffer IDs to cross-PD stable IDs (major architectural decision)

---

## Proof Verification

```
SAFE_LOCAL_PATCH: YES — all violations are shell-local docs gaps or minor naming issues
FIX_FIRST: NO — no blocking correctness bug found beyond K2A (already fixed)
BLOCKED_STOP_FIRST: NO — no namespace issue crosses ABI, PDX, or PKEY

IPCPKU_MAP source path: /home/xirtus_arch/Documents/microkernel/IPCPKU_MAP.md
namespace source-of-truth: IPCPKU_MAP defines only PKEY + PDX slots; all else is shell-local undocumented
violation table: 6 violations found; 1 already resolved (K2A), 5 remaining (V1-V5)
corrected namespace table: all tiers enumerated above; K2A dynamic range 1001-1016 is correct
K2 patch sequence: K2B (docs) → K2C (seed coherence, real Claude) → K2D (small constants) → K2E (IPCPKU_MAP addendum)
deepseekclaude: K2B and K2D items 1+2 safe; everything else needs real Claude
STOP FIRST: new opcodes, new PDX slots, PKEY changes, real Collar grant semantics, PD migration
```
