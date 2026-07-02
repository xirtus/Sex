# RUNTIME_SMOKE_POST_CHROME_V1

**Verdict: PASS RUNTIME — 0 faults, all invariants intact.**
**Date:** 2026-05-16
**Scope:** Post-chrome / post-launch / post-Atlas / post-Bell Bridge status sprint smoke checkpoint.

---

## 1. Build

```
./scripts/entrypoint_build.sh
[SEXOS ENTRYPOINT] success
```

ISO produced: `sexos-v1.0.0.iso` (1919 sectors).

---

## 2. Daily Proof

```
SEXOS_BELL_BRIDGE_STUB_PROOF=1 ./scripts/run_daily_driver_proof.sh
```

| Metric | Value |
|--------|-------|
| Gates | **87/87 PASS** |
| Failures | 0 |
| Skipped | 0 |
| Faults | 0 |
| Log lines | 8014 |

Key gate results (new/recent):

| Gate | Result |
|------|--------|
| frame_chrome_model | PASS (scenes=1 frames=3 tabs=3) |
| frame_rim_markers | PASS (3 frames rim=dim/focused render=0) |
| frame_rim_visual | PASS (3 frames rendered alpha=0 blur=0) |
| frame_lights_stub | PASS (red=disabled yellow/green=available) |
| browser_stub | PASS (fetched=0 engine=0) |
| browser_placeholder | PASS (launch request sent) |
| atlas_scene_stub | PASS (1 scene visual=0 thumbnails=0) |
| silk_glass_color | PASS (7 colors changed no alpha/blur) |
| bell_events | PASS |
| bell_app_events | PASS (8 app events emitted) |
| bell_workflow_events | PASS (4 workflow events) |
| bell_delivery_audit | PASS |
| spindle_slot_shell | PASS (SLOT_SHELL route exists) |
| launcher_multi_exec | PASS (7/7 apps, 7 execs) |
| faults_zero | PASS (0 fault markers) |

---

## 3. QEMU Smoke

Booted for 18 seconds, log captured at `/tmp/sexos_runtime_smoke_post_chrome_v1_qemu.log`.

| Metric | Value |
|--------|-------|
| Log lines | 7594 |
| Clock ticks | 39 |
| Faults | **0** (#PF=0, #GP=0, fault.kill=0, KERNEL PANIC=0) |
| PDs spawned | 12 (display, drive, shell, input, usb, silkbar, linen, store, quil, bell, files, spindle) |

---

## 4. Truth Invariant Verification

### 4.1 Frame Chrome / Rim / Lights

| Invariant | Observed | Status |
|-----------|----------|--------|
| Frame chrome model present | scenes=1 frames=3 tabs=3 | ✅ |
| Frame rim rendered | 3 frames, focused=1 dim=2, alpha=0 blur=0 | ✅ |
| Frame lights red disabled | red=disabled, close_allowed=0, close_impl=0 | ✅ |
| Frame lights yellow/green available | yellow=available green=available (3 frames) | ✅ |
| Frame lights visual=0 pointer=0 | visual=0 pointer=0 | ✅ |

### 4.2 SLOT_SHELL Launch

| Invariant | Observed | Status |
|-----------|----------|--------|
| SLOT_SHELL grants issued | kernel→PDs 3,4,12 (shell, input, spindle) | ✅ |
| Launch exec rows present | spindle_launch_exec PASS, launcher_multi_exec 7/7 | ✅ |
| App registry launch_exec truth intact | Spindle=1 (self), Quil/Linen=1 (SLOT_SHELL), Bell/Atlas=0 | ✅ |

### 4.3 Browser Placeholder

| Invariant | Observed | Status |
|-----------|----------|--------|
| network=0 | browser.stub.blocker: network=0 | ✅ |
| engine=0 | browser.stub.blocker: engine=0 | ✅ |
| fetched=0 | browser_stub gate: fetched=0 | ✅ |
| launch_exec=0 | app.registry: launch=none launch_exec=0 | ✅ |
| Phase freeze intact | phase=0 stub_foundation only, phases 1-4 planned | ✅ |

### 4.4 Atlas Scene Status

| Invariant | Observed | Status |
|-----------|----------|--------|
| visual=0 | silk.atlas.summary: visual=0 | ✅ |
| thumbnails=0 | silk.atlas.summary: thumbnails=0 | ✅ |
| pointer=0 | silk.atlas.mode: pointer=0 | ✅ |
| drag=0 | silk.atlas.mode: drag=0 | ✅ |
| scenes=1 | atlas_scene_stub gate: scenes=1 | ✅ |

### 4.5 Bell Bridge Status

| Invariant | Observed | Status |
|-----------|----------|--------|
| phase=1 | bell.bridge.status.stub phase=1 | ✅ |
| ipc=0 | bell.bridge.status.stub ipc=0 | ✅ |
| launch=0 | bell.bridge.status.stub launch=0 | ✅ |
| focus=0 | bell.bridge.status.stub focus=0 | ✅ |
| render=0 | bell.bridge.status.stub render=0 | ✅ |
| ok=1 | bell.bridge.status.ready ok=1 | ✅ |

### 4.6 Bell V1 Backward Compatibility

| Invariant | Observed | Status |
|-----------|----------|--------|
| Bell boot marker present | [bell.boot] emitted | ✅ |
| SilkBar Bell presence active | silkbar→bell slot=12 cap granted | ✅ |
| Spindle Bell bridge proofs | All 7 stages pass, [spindle.bell.proof.done] ok=1 | ✅ |
| Bell event delivery | bell_events PASS, bell_delivery_audit PASS | ✅ |

### 4.7 Silk Glass Colors

| Invariant | Observed | Status |
|-----------|----------|--------|
| 7 colors changed | silk.glass.safe_color_pass.done changed=7 | ✅ |
| No alpha/blur changes | colors only, no alpha/blur in rim proof | ✅ |

---

## 5. Fault Count

**0 faults** across all verification layers:

| Layer | Fault Count |
|-------|-------------|
| Build | 0 |
| Daily proof | 0 |
| QEMU smoke | 0 (#PF=0, #GP=0, fault.kill=0, KERNEL PANIC=0) |

---

## 6. Visual Observation

QEMU booted with `-display gtk`. Expected boot sequence observed:
- Dark glass-panel desktop with SilkBar at top
- Frame rim borders visible (focused + dim)
- SilkBar clock ticking
- Bell dot rendered (gold, count badge from demo event)
- No visual regressions from prior boot

---

## 7. Key Marker Summary

```
[SEXOS ENTRYPOINT] success                                       ← build
[bell.boot]                                                      ← Bell V1
[bell.bridge.status.stub] phase=1 ipc=0 launch=0 focus=0 render=0  ← new
[bell.bridge.status.ready] ok=1                                  ← new
[silk.frame.rim.visual.proof.done] ok=1 rendered=3               ← frame rim
[silk.frame.lights.status_stub.done] ok=1 frames=3               ← frame lights
[silk.atlas.status_stub.done] ok=1 scenes=1 visual=0             ← atlas
[browser.stub.blocker] network=0 engine=0                        ← browser
[silk.glass.safe_color_pass.done] ok=1 changed=7                 ← glass colors
[spindle.bell.proof.done] ok=1                                   ← bell bridge
DAILY-DRIVER MASTER GATE V32: PASS (87 gates, 0 faults)          ← daily proof
```

---

## 8. Handoff Path

```
docs/handoff/RUNTIME_SMOKE_POST_CHROME_V1.md
```

---

## 9. Commit Command

```bash
git add docs/handoff/RUNTIME_SMOKE_POST_CHROME_V1.md
git commit -m "docs(runtime): post-chrome smoke V1 — 87 gates 0 faults"
```

---

*End of RUNTIME_SMOKE_POST_CHROME_V1.md*
