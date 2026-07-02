# CANONICAL_TRACK_INDEX_V1

**Status:** Active — dependency map for SexOS/Silk DE canonical plan documents.
**Purpose:** One compact reference so agents stop guessing track order, dependency gates, and coverage status.

## Canonical Plan Documents

| Doc | Track | Lines | Scans 1-8 | Phase Ladder | Status |
|-----|-------|-------|-----------|--------------|--------|
| `A_COMPOSITOR_LIFECYCLE_PLAN_V1.md` | A: Compositor Lifecycle | 298 | ✅ | A1-A8 | Approved |
| `B_APP_LAUNCH_SESSION_RESTORE_PLAN_V1.md` | B: App Launch + Session Restore | 328 | ✅ | B1-B7 | Approved |
| `C_TOUCHPAD_GESTURES_PLAN_V1.md` | C: Touchpad Gestures | 283 | ✅ | C1-C7 | Approved |
| `D_ACCESSIBILITY_STACK_PLAN_V1.md` | D: Accessibility Stack | 279 | ✅ | D1-D8 | Approved |
| `PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` | E: Persistent Storage | 577 | ✅ | E1-E11 | Approved |
| `LINEN_DOCUMENT_LIFECYCLE_PLAN_V1.md` | F: Linen Document Lifecycle | 684 | ✅ | F1-F10 | Approved |
| `SEXAUDIO_HARP_PHASE_PLAN_V1.md` | SexAudio/Harp | 530 | ✅ | SA1-SA11 | Approved |
| `THEREMIN_SYSTEM_SOUND_ENGINE_PLAN_V1.md` | Theremin | 515 | ✅ | T1-T8 | Approved |
| `handoff/QUIL_SURFACE_STUB_V1.md` | Quil (stub) | 323 | ✅ | — | Approved (stub only) |

## Historical / Reference-Only Documents (Not Build Authority)

| Doc | Purpose | Note |
|-----|---------|------|
| `phase25-compositor.md` (771 lines) | Historical compositor design with PDX/IPC transport | Reference only for PDX opcode conventions. Not lifecycle authority. |
| `rapid/PHASE_00_BASELINE_PROOF_GATES.md` | Baseline proof gate design | Superseded by canonical A-F docs. Keep for proof marker conventions. |
| `rapid/PHASE_01_SILK_DISPLAY_CONTRACT_RENDER.md` | Display contract render spec | Superseded by canonical A doc sexdisplay rules. |
| `rapid/PHASE_02_SHELL_SURFACE_OWNERSHIP_SCENE_FRAME_TAB.md` | Shell surface ownership | ~70% complete but superseded by A doc lifecycle FSM. |
| `rapid/PHASE_03_INPUT_COMPLETION_USB_MOUSE.md` | USB input completion | Reference for C/Track C gesture pipeline design. |
| `rapid/PHASE_04_LINEN_FILE_OBJECT_BROWSER.md` | Linen file/object browser | Early reference for F track. |
| `rapid/PHASE_05_QUIL_LANGUAGE_WORKSTATION.md` | Quil language workstation | Quil scope not yet canonicalized. |
| `rapid/PHASE_06_MESH_CAPABILITY_GRAPH.md` | Mesh capability graph | Reference for G/H/I design (future). |
| `rapid/PHASE_07_APP_LAUNCH_PACKAGE_PATH.md` | App launch package path | Reference for B track launch/restore. |
| `rapid/PHASE_08_NETWORK_DEVICE_REALITY.md` | Network device reality | Reference for future network tracks. |
| `rapid/PHASE_09_BELL_NOTIFICATIONS_SETTINGS.md` | Bell notifications/settings | Reference for future G/H/I design. |

## Dependency Gates

```
 A (Compositor Lifecycle)
 ├── A1-A4 must be complete before B implementation
 ├── A4 must be complete before C gesture target validation
 ├── A4 must be complete before D keyboard navigation
 └── Foundation for all surface lifecycle, focus validity, lifecycle generation
 B (App Launch + Session Restore)
 ├── Depends on A1-A4 (focus validity, lifecycle FSM)
 ├── Durable restore waits E gates (E5-E9)
 └── Document restore waits F (OpenIntent validation)
 C (Touchpad Gestures)
 ├── Depends on A4 (focus validity guards)
 ├── C7/C8 gate verifies D input alternatives before gesture customization
 └── USB HID pointer producer must exist (C1 audit)
 D (Accessibility Stack)
 ├── Depends on A4 (focus validity for keyboard nav)
 ├── D provides input alternatives for C (C7/C8 gate)
 └── D gates keybinding customization in A/B/C/Quil/Linen
 E (Persistent Storage Maturity)
 ├── Foundation for durable persistence in B (SceneRestoreJournal)
 ├── Foundation for durable persistence in F (document storage)
 └── Foundation for preference persistence in all tracks (Scan 8)
 F (Linen Document Lifecycle)
 └── Depends on E (storage trust layer)
 SexAudio/Harp
 ├── Foundation for Theremin (sound intent routing/mixing)
 └── SA7 buffer transport gate before any audio implementation
 Theremin
 └── Depends on SexAudio (sound intent routing through SexAudio)
 Quil (stub only)
 └── No formal dependency yet — stub defines surface identity only
```

## Blocked-Before Rules

| Rule | Blocked | Blocks Until |
|------|---------|-------------|
| No lifecycle implementation before A1-A4 | B, C, D | A1 audit + A2 FSM + A3 model + A4 focus guards complete |
| No launch/restore implementation before A1-A4 | B | A1-A4 complete |
| No gesture implementation before C1 audit | C | NormalizedPointerEvent delivery confirmed |
| No gesture customization before D alternatives | C Scan 8 | D input alternatives verified |
| No accessibility implementation before A4 | D | A4 focus validity guards complete |
| No gesture customization without C7/C8 gate | C, D | C7 proof scenarios + D input alternatives verified |
| No durable storage before E gates | B, F, all Scan 8 prefs | E5-E9 storage maturity |
| No document restore before F | B | F OpenIntent canon complete |
| No audio implementation before SA7 | SexAudio, Theremin | SA7 buffer transport gate approved |
| No Theremin sound before SexAudio | Theremin | SexAudio exists + accepts Theremin client |
| No package trust before G gate | B (launch trust) | G track designed |
| No kernel/PDX ABI edits (any track) | All | Explicit handoff with STOP FIRST override |

## Implementation Order (Recommended)

```
Phase 1: A1-A4 (audit → FSM → model → focus)
Phase 2: B1-B2 (launch audit → identity/manifest spec)
Phase 3: C1-C3 (input audit → boundary → FSM spec)
Phase 4: D1-D2 (accessibility audit → semantic role spec)
Phase 5: B3-B4 (launch intent → runtime instance model)
Phase 6: C4-C5 (shell gesture model → target validity)
Phase 7: D3-D4 (keyboard nav → narration event log)
Phase 8: A5-A8 (frame lights → tombstones → conformance → proof)
Phase 9: B5-B7 (restore journal → validation → integration)
Phase 10: C6-C7 (intent dispatch → proof scenarios)
Phase 11: D5-D8 (input alternatives → policy → tree → wire)
Phase 12: E1-E11 (storage maturity — parallel track)
Phase 13: F1-F10 (linen lifecycle — after E)
Phase 14: SexAudio SA1-SA11 (after A foundation)
Phase 15: Theremin T1-T8 (after SexAudio)
Phase 16: G/H/I (future — package trust, crash log, dev cockpit)
```

## Docs Needing Future Compression

| Doc | Current Lines | Target | Notes |
|-----|--------------|--------|-------|
| `LINEN_DOCUMENT_LIFECYCLE_PLAN_V1.md` | 684 | 500-650 | Already compressed from 834. Could lose another 34 with tighter §22/§23 merging. |
| `SEXAUDIO_HARP_PHASE_PLAN_V1.md` | 530 | 400-500 | SA7 transport gate and SA1-SA11 phase details could compress. |
| `THEREMIN_SYSTEM_SOUND_ENGINE_PLAN_V1.md` | 515 | 400-500 | Physical model preset tables (T6) in handoff could allow main doc compression. |

## Scan 7/8 Coverage Status

| Track | Scan 7 (Exceeded Hypothesis) | Scan 8 (Customization) |
|-------|------------------------------|------------------------|
| A: Compositor Lifecycle | ✅ 10 rows | ✅ 10 domains, 11 boundaries, 9 scenarios |
| B: App Launch + Session Restore | ✅ 10 rows | ✅ 10 domains, 12 boundaries, 10 scenarios |
| C: Touchpad Gestures | ✅ 10 rows | ✅ 10 domains, 11 boundaries, 10 scenarios |
| D: Accessibility Stack | ✅ 10 rows | ✅ 10 domains, 11 boundaries, 10 scenarios |
| E: Persistent Storage | ✅ 7 rows | ✅ 9 domains, 17 boundaries, 13 scenarios |
| F: Linen Document Lifecycle | ✅ 10+7 rows | ✅ 14 domains, 21 boundaries |
| SexAudio/Harp | ✅ 10 rows | ✅ 10 domains, 12 boundaries, 10 scenarios |
| Theremin | ✅ 10 rows | ✅ 10 domains, 11 boundaries, 10 scenarios |
| Quil (stub) | ✅ (merged) | ✅ 9 domains, 15 boundaries, 10 scenarios |

## Key Principles

- **Shell owns policy** in all tracks. sexdisplay renders shell-provided visual state only. SexAudio routes/mixes audio only. Apps never force lifecycle/focus/capability.
- **No kernel/PDX ABI edits** without explicit handoff and STOP FIRST override.
- **No POSIX assumptions**: no PID/env/CWD/fd/argv, no .desktop, no AT-SPI, no ALSA/PulseAudio, no libinput/evdev.
- **no_std Rust** throughout. No std threads/sleep/time. No float DSP in Theremin.
- **Static arrays only** — no Vec, no heap allocation for FSM/state/tables.
- **Proof markers required** for all lifecycle transitions, launch/restore operations, gesture commits, accessibility narration, audio routing, and sound generation.
- **E gates before durable persistence** — all Scan 8 preferences are memory-only until E5-E9.
- **D gates keybinding customization** across all tracks — no shortcut remapping without D accessibility + shortcut/conflict audit.
