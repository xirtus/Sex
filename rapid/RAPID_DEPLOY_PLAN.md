# Rapid Deploy Plan — 11 Phases (Revised)

**Rule:** Bundle by invariant family, not by excitement. One phase may include many files only if ownership is aligned. No kernel/ABI/sex-pdx edits unless STOP FIRST. Every phase ends with build + boot + runtime proof + fault scan + handoff.

## Consolidation


## Phase Map

| # | Phase | Ownership | Dependencies | Parallel With |
|---|-------|-----------|-------------|---------------|
| 00 | Baseline Proof + Gates | Shared (gates) | None | 01, 02 |
| 01 | Silk Display Contract + Render | silkbar/sexdisplay/silkbar-model | None | 00, 02 |
| 02 | Shell Surface Ownership + Scene/Frame/Tab | silk-shell | None | 00, 01 |
| 03 | **Glass Chrome (alpha transparency)** | **sexdisplay/silk-shell** | **None (sexdisplay only)** | **02 finish** |
| 04 | **ChromeTemplate V1 (data-driven)** | **silk-shell/sexdisplay** | **GLASS, 02 (frame model)** | **—** |
| 05 | **Bell: Attention Firewall** | **Bell/SilkBar/Quil** | **01, 02 (display + shell)** | **04, 05** |
| 06 | **Linen: Object Layer** | **sexfiles/linen/silk-shell** | **None (hardcoded data first)** | **09, 05** |
| 07 | Network + Device Reality | sexnet/sexusb | 03B (USB) for devices; net UI: None | 07, 10 |
| 08 | App Launch + Package Path | sexshop/silk-shell | 06 (Collar capability flow) | 06, 08 |
**09, 04** |
| 10 | **Mesh + Collar: Living Graph + Authority** | **Mesh/Collar/Quil** | **02 (scene model), 05 (Quil visualization)** | **07, 08** |
| 11 | **Quil: Language Workstation** | **Quil/Linen/Mesh/Collar** | **04 (Linen objects)** | 
| 12 | Input Completion + USB Mouse | silk-shell/sexusb/sexinput | 3A: 02 (partial), 3B: None | 07, 10 |
| 13 | Core App Suite + Compatibility | Quil/sexfiles | 05 (Quil surface lifecycle) | 03, 08 |
| 14 | Hardening + Persistence + Release | All servers | Everything above | Sequential |

## Visual Foundation (GLASS + CHROME_TEMPLATE)

Two phases that make the OS feel polished before any revolutionary features are built:

| Phase | What It Does | Effort | Why Revolutionary |
|-------|-------------|--------|-------------------|
| **GLASS_V1** | Alpha transparency on window chrome | ~6h | Enables semi-transparent top bar, rim, tabs. Infrastructure for all glass effects. |
| **CHROME_TEMPLATE_V1** | Data-driven chrome + animation | ~17h | **Chrome and animation parameters are a fixed-size struct, not code.** Hot-swap without rebuild. 4 profiles with distinct glass + animation. Generic animation engine driven by template data. |

## Revolutionary Core (Phases 09→04→05→06)

| Phase | What It Does | Why Revolutionary |
|-------|-------------|-------------------|
| **Bell** | Attention firewall | Capability-scoped urgency, notification lanes, attention budget, DEV mode, sender identity verification. Not a notification daemon — the user's attention firewall. |
| **Linen** | Object layer | Objects are first-class OS citizens with types, capabilities, provenance, project graphs. Not a file manager — the semantic graph over user data. |
| **Quil** | Language workstation | Everything from OnlyOffice to Kate Coder, in different "modes" 0. Code Mode, for any file with standards, html js etc. 1. Sex Mode: coding specifically restricted and enhanced for Sex specifications: PDX call graph, capability slots, no_std scanner, ABI drift detector beside code. Not an IDE — a workstation aware of the OS's laws. 2. Office Mode, 3. Design Mode 4. Business Mode (spreadsheets etc),  |
| **Mesh (Graph) & Collar (Wallet)** | Living graph + authority | Nervous system + immune system. Temporal graph, borrow-checker capabilities, pattern bounds anomaly detection, promptless consent. Not monitoring + permissions — self-awareness by construction. |

## Table Stakes (Phases 07→03→10→08→11)

These are important but conventional — every OS has them. Do them after the revolutionary core is proven.

## Key Improvements (from 3-pass review)

### What changed in every document:
1. **"What Already Exists" section** — prevents rebuilding what's already done
2. **"Smallest First Step"** — identifies the minimal commit that proves the concept
3. **"Exit Criteria" (Done Checklist)** — verifiable pass/fail items, not subjective
4. **"Dependencies" + "Parallel With"** — enables parallel execution across ownership domains
5. **"Risks & Mitigations"** — identifies showstoppers before they happen
6. **"Testing Strategy"** — how to verify each bundle
7. **"Efficiency Opportunity"** — consolidation/shortcut specific to each phase

### Major consolidations:
- **Phase merged**: Mesh (self-awareness) + Collar (capability conscience) are now designed as a single living system — the nervous system + immune system of the OS. They share the temporal graph data structure, anomaly detection, and capability provenance model. This is not a "system monitor" + "permission manager" — it is a **new category of OS capability** that no existing system has.
- **Phase explicit parallelism**: Shell input policy and USB driver are fully parallelizable — no shared files, no ordering dependency.
- **Phase decoupled from storage**: Linen UI uses hardcoded data first, integrates sexfiles later.
- **Phase colored-block rendering**: Skips text glyph pipeline entirely in V1. Functional now > beautiful later.
- **Phase terminal-first**: Terminal is the killer app. Calculator/Notes/Media are secondary.
- **Phase split into 3 subphases**: Harden (audit/crash), Persist (sexstore), Release (installer). Can be done incrementally.

### Reliability improvements:
- Every phase has **verified exit criteria** (build + boot + markers + faults = 0)
- Every risk has a **specific mitigation**, not just acknowledgment
- Every phase identifies what **already exists** so we don't rebuild
- Gate scripts from Phase 0 protect all subsequent phases from regression
- "Smallest First Step" ensures each phase produces value before scope expands

## Forbidden Boundaries

The following are never bundled together in one phase:
- XHCI + HID + gestures + compositor
- Shell + storage + kernel
- Settings + persistence + installer
- App framework + compatibility layer
- Observable graph + authority enforcement

## Parallel Execution Strategy

```
Day 1:   Phase 00  Phase 01  Phase 02              (all parallel — finish remaining work)
Day 1:     GLASS_V1  CHROME_TEMPLATE_V1              (glass blending first, then data-driven chrome + animations)
Day 1:   Phase 09  Phase 04                        (parallel — Bell + Linen, both pure PDX servers)
Day 2-3:   Phase 05  Phase 04 (cont'd)               (Quil Language WS + Linen integration)
Day 3:   Phase 406                                  (Mesh + Collar — the capstone)
Day 4: Phase 07  Phase 03                        (App Launch + USB — parallel, independent)
Day 4: Phase 10  Phase 08                        (App Suite + Network — parallel)
Day 4: Phase 11                                  (Hardening — sequential subphases)
```

Total: ~15 weeks with parallelization. Glass + data-driven chrome in week 3. Revolutionary core (Bell→Linen→Quil→Mesh+Collar) delivered by week 9.

## AI-Assisted Development

These phases are ideally suited for AI code generation:

| Phase | AI Speedup | Why |
|-------|-----------|-----|
| **GLASS_V1** | 2-3× | Small targeted changes (alpha_blend, blend_chrome, 6 call sites). AI generates the exact code. |
| **CHROME_TEMPLATE_V1** | 2-3× | Struct definition, static presets, generic animation engine (tween + ease + tick). AI generates the template system and animation core. Human wires into minimize/zoom dispatch. |
| **09 Bell** | 3-4× | Pure PDX server pattern — ring buffer, opcode dispatch, policy structs |
| **04 Linen** | 3-4× | CRUD server with fixed-size Object structs, capability-gated access |
| **05 Quil** | 2-3× | Surface lifecycle, mode dispatch, colored-block rendering — well-understood patterns |
| **06 Mesh+Collar** | 1.5-2× | Novel architecture — design is human-driven, implementation is AI-assisted |

Estimated AI-assisted total: ~190h raw implementation → ~270h with debugging/QA → 10-15 weeks full-time.
