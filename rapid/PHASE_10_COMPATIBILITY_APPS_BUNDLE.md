# PHASE 10: Core App Suite + Compatibility

## Goal
Build the first real application suite for daily use. Establish Linux/binary compatibility path (design only, not ship). Apps must be useful enough that a developer can live in SexOS for a workday.

## Ownership
- **Quil** (exclusive): terminal, calculator, notes, media viewer apps
- **sexdrive/sexfiles** (integration): file open/save for apps
- **Tuxedo** (compat design): Linux syscall bridge evaluation (design document only)
- **Mesh/Collar** (read-only): compatibility status tracking

## What Already Exists
- Quil surface lifecycle exists (Phase 5 creates Quil as development workstation)
- sexfiles provides file read/write (Phase 4)
- sexdisplay renders flat ARGB surfaces (all apps use standard surface chrome)
- Keyboard input routing to focused surface exists (Phase 3A)
- No app framework or SDK exists beyond the basic surface/chrome pattern
- No terminal emulator (PTY-like surface communicating with shell process)

## Bundle

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| **Terminal** | PTY surface: keyboard input → shell process → output rendered | 10h | HIGH (most important app) |
| **Calculator** | Basic arithmetic: buttons rendered as colored grid, compute in shell | 4h | Medium |
| **Notes** | Text note app: edit buffer, save/load via sexfiles, flat ARGB rendering | 6h | Medium |
| **Media/image viewer** | Render static image from pixel data (PNM/PPM format, no PNG decode) | 4h | Low |
| **Tuxedo compatibility design** | Linux syscall bridge evaluation document, not implementation | 3h | Low |
| **Web runtime decision** | Evaluate: embedded web view vs open protocol vs defer | 2h | Low |

## Smallest First Step
Build the Terminal. It's the single most important app for daily-driver readiness. A terminal needs:
1. Keyboard input routing to app surface (exists from Phase 5)
2. A shell process to communicate with (sexos shell via PDX or a simple command executor)
3. Output rendered as colored rectangles on the surface (same colored-block technique from Phase 5)

Prove: type "ls" → output appears as colored blocks. That's a usable terminal.

## Dependencies
- **Blocking**: Phase 5 (Quil surface lifecycle, keyboard routing)
- **Blocked by**: Phase 5 for app surface pattern, Phase 4 for file save/load
- **Can parallelize with**: Phase 8 (network), Phase 9 (Bell/settings)
- **Key insight**: Apps don't need network, notifications, or settings to be useful. Terminal + Calculator + Notes can be built immediately after Phase 5.

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Terminal needs a shell process (process spawn) | High | HIGH | V1: Terminal communicates with a simple command executor running in silk-shell's PD. No process spawn — just a command string → output string loop. True process spawn deferred to Phase 7 (app launch). |
| No text glyph rendering (can't show command output) | High | HIGH | Use colored-block encoding: green block = character, gray = background. Each character is a colored rectangle. This is primitive but functional. A full font renderer is Phase 12+ work. |
| Calculator needs button hit targets | Medium | Low | Buttons are rendered as colored rectangles in the surface chrome zone. silk-shell hit-test dispatches clicks to the calculator surface. Calculator determines which button based on x/y offset. |
| Media viewer needs image decode | Medium | Medium | V1: PPM/PBM format only (trivially parseable). Defer PNG/JPEG decode to later. If image format is unsupported, show "cannot decode" message. |
| Linux compatibility (Tuxedo) distracts from native apps | High | Medium | Tuxedo is a design document only — no implementation. The goal is to evaluate feasibility, not ship Linux binary support. Defer implementation decision to after Phase 12. |

## Exit Criteria (Done Checklist)
- [ ] Terminal: keyboard input → command → output displayed as colored blocks
- [ ] Terminal: cursor follows input, scrolling (bounded buffer, last N lines visible)
- [ ] Calculator: 4 operations (+, -, ×, ÷), clickable buttons, result displayed
- [ ] Notes: create, edit, save via sexfiles, load on reopen
- [ ] Media viewer: displays PPM/PBM images from sexfiles
- [ ] Tuxedo compatibility design document created (not implemented)
- [ ] All apps use standard surface lifecycle (0xEC create, 0xFD tab info, focus)
- [ ] Build passes. Boot passes. No panic for any app.

## Testing Strategy
- **Terminal**: Type a known command, verify expected output pattern. Test with empty input, long input (overflow), rapid typing.
- **Calculator**: Click each button, verify display updates. Test edge cases: division by zero, overflow, negative results.
- **Notes**: Create note, save, close, reopen, verify content preserved.
- **Media viewer**: Load a known PPM image, verify correct colored blocks at expected positions.
- **Integration**: Launch terminal from launcher (Phase 7). Save calculator result to notes. Open saved note and verify.

## Efficiency Opportunity
**Terminal IS the killer app for daily-driver readiness.** If a developer can open a terminal, run build commands, and see output, they can develop SexOS from within SexOS. Prioritize terminal above all other apps. Calculator and Notes are nice-to-haves that can be deferred.

**Skip image/media viewer in V1.** No one will use SexOS for photo editing. Terminal + sexfiles + Quil editor covers 90% of a developer's daily needs.

## Completeness Gain
Apps/user utilities: **35–50% → 65–80%** (terminal + sexfiles + Quil editor is sufficient for development)

## Files Changed
- `servers/quil/src/main.rs` (Terminal mode, Calculator panel, Notes panel)
- `servers/silk-shell/src/main.rs` (app keyboard forwarding, command executor integration)
- `servers/sexfiles/src/main.rs` (Notes save/load via sexfiles)
- `docs/handoff/TUXEDO_COMPATIBILITY_DESIGN_V1.md` (new — design document only)

## Forbidden
- Kernel compatibility layer
- POSIX assumptions in core servers
- GPL-licensed compatibility code
- Full PNG/JPEG image decoder (PPM only)
- Real process spawn (use in-PD command executor)
- Broad refactor

## Next Phase
PHASE_11_HARDENING_PERSISTENCE_RELEASE.md
