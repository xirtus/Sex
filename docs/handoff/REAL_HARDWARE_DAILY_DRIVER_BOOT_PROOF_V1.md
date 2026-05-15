# REAL_HARDWARE_DAILY_DRIVER_BOOT_PROOF_V1

Date: 2026-05-15
Status: PLANNING (docs-only, zero source changes)
Scope: docs/ only — real-hardware boot checklist and manual proof sequence

## 1. PASS/STOP FIRST

| Status | Meaning |
|--------|---------|
| **PASS** | Checklist and proof sequence documented. Zero source changes. |
| **STOP FIRST** | No code edits. No kernel/ABI/USB/display changes. This is a boot plan only. |

### Architecture Constraints (unconditionally preserved)

| Constraint | Why |
|-----------|-----|
| `no_std` Rust microkernel | No libc, no POSIX, no threads. PDX-only IPC. |
| MPK/PKU/PKEY isolation | Per-server memory protection domains. No shared heaps. |
| No kernel edits | Kernel is stable. Hardware issues are NOT kernel bugs until proven. |
| No sex-pdx/ABI edits | PDX opcodes, slots, wire format frozen. |
| No sexusb/sexinput/sexdisplay edits | Drivers are stable. Debug at protocol/log level first. |
| **Do NOT edit source to debug hardware** | Reproduce in QEMU preflight first. Capture photo/video/log. Create a separate hardware-blocker handoff if failure occurs. Never hot-patch source on hardware. |

## 2. Current Baseline

| Metric | Value |
|--------|-------|
| QEMU proof gate count | **18/18 PASS** |
| Faults (QEMU) | **0** |
| Proof env vars | **41** |
| ISO size | ~4 MB |
| Surface IDs | 153=Spindle, 200=Linen, 201=Quil, 202=Mesh, 203=Collar, 204=Bell, 151=Atlas |
| Key codes | 0x29=backtick (palette), F10=Atlas, J/K=nav, Enter=execute |
| Pointer/slot2 | Deferred — not tested on hardware |

## 3. QEMU Preflight (MANDATORY — run before every hardware boot)

```bash
# 0. Verify clean git state (no uncommitted hardware hacks)
git status --short
# Must show zero modified source files (docs/ and scripts/ changes OK).
# If servers/ or crates/ show modified: STASH OR COMMIT before proceeding.

# 1. QEMU build + boot + gate scan (full proof profile)
./scripts/run_daily_driver_proof.sh /tmp/sexos_pre_hardware_daily_driver.log

# 2. Verify results
grep -E "PASS gates|FAIL gates|FINAL|fault" /tmp/sexos_pre_hardware_daily_driver.log
```

**Required output:**
```
PASS gates: 18
FAIL gates: 0
FINAL: PASS (18 gates proved, 0 skipped, 0 faults)
faults_zero: PASS   0 fault markers
```

| Check | Threshold | Action if fail |
|-------|-----------|---------------|
| PASS gates | 18 exactly | **STOP.** Fix QEMU before touching hardware. |
| FAIL gates | 0 | **STOP.** Any FAIL is a regression. |
| Faults | 0 | **STOP.** Kernel fault = build broken. |
| Git status | No source diffs | Stash/commit. Never boot hardware with dirty source. |

**If any check fails: DO NOT boot hardware.** Resolve in QEMU first.

## 4. Hardware Boot Checklist

### 4.1 Prepare ISO

```bash
# Build from current master with full proof profile
./scripts/run_daily_driver_proof.sh /tmp/sexos_pre_hardware_v1.log

# Verify: 18/18 PASS, faults=0
grep "FINAL" /tmp/sexos_pre_hardware_v1.log
# Expected: FINAL: PASS (18 gates proved, 0 skipped, 0 faults)

# Copy ISO to Ventoy USB
cp sexos-v1.0.0.iso /mnt/ventoy/sexos-v1.0.0.iso
sync
umount /mnt/ventoy
```

### 4.2 Boot Sequence

| Step | Action | Expected |
|------|--------|----------|
| 1 | Insert Ventoy USB | — |
| 2 | Power on Alienware, press F12 | Boot menu |
| 3 | Select USB (UEFI) | Ventoy menu |
| 4 | Select `sexos-v1.0.0.iso` | Limine bootloader → SexOS |
| 5 | Wait ~5s | Screen changes from black → background gradient |

### 4.3 First Visual Signs (expected on screen)

| Sign | What to look for | Location |
|------|-----------------|----------|
| Background gradient | Dark blue → deeper blue (top→bottom) | Full screen |
| SilkBar panel | Semi-translucent strip | Top of screen, ~50px tall |
| Clock digits | `10:42` or silkbar-clock time | Top-right of bar |
| Workspace indicators | Colored tabs | Center of bar |
| Chip indicators | Small colored dots (Net, Wifi, Battery) | Right of center |
| Bell dot | Gold/amber dot (if events) | Between Battery and Clock |
| Launcher icon | Blue glass square | Top-left of bar |
| Phase 5 indicators | Tiny colored dots: active app (left), tint swatch (right), palette dot (right) | Bar area |
| Cursor | Blinking or solid text cursor | At focused surface |

### 4.4 No Serial Capture Fallback

**Hardware has no serial port exposed for logging.** All verification is visual/keyboard behavioral:

- Record screen with phone camera for post-hoc analysis
- Note exact sequence of key presses and observed reactions
- If screen is black: note exact point of failure (Limine menu? after selection? immediate?)

## 5. Manual Keyboard-Only Proof Sequence

### 5.1 Phase A: Boot + Bar Verification

| Step | Key | Expected behavior | Pass? |
|------|-----|-------------------|-------|
| A1 | (none) | Screen shows background gradient + SilkBar at top | ☐ |
| A2 | (none) | Clock visible, workspaces visible, chips visible | ☐ |
| A3 | (none) | Phase 5 indicators visible (app dot, tint swatch, palette dot) | ☐ |
| A4 | (none) | No glitches, tearing, or frozen pixels | ☐ |

### 5.2 Phase B: Command Palette

| Step | Key | Expected behavior | Pass? |
|------|-----|-------------------|-------|
| B1 | `` ` `` (backtick) | Command palette overlay appears | ☐ |
| B2 | (visual) | Palette shows rows: Spindle, Quil, Linen, Atlas, Bell, Collar, Mesh, etc. | ☐ |
| B3 | `J` (repeat) | Selection moves down through items | ☐ |
| B4 | `K` (repeat) | Selection moves up through items | ☐ |
| B5 | `` ` `` (backtick) | Palette closes | ☐ |
| B6 | `` ` `` again | Palette re-opens | ☐ |

### 5.3 Phase C: Launch Spindle (idx 0)

| Step | Key | Expected behavior | Pass? |
|------|-----|-------------------|-------|
| C1 | `` ` `` (open palette) | Palette visible | ☐ |
| C2 | `J`/`K` → select Spindle | Spindle row highlighted | ☐ |
| C3 | `Enter` | Spindle surface appears | ☐ |
| C4 | (visual) | Spindle tile/window visible on desktop | ☐ |
| C5 | (visual) | Phase 5 active app dot changes to Spindle blue | ☐ |
| C6 | Type `h` | Spindle help text appears in Spindle surface | ☐ |
| C7 | Type `d` | Spindle daily summary appears | ☐ |
| C8 | Type `b` | Spindle status/blockers appears | ☐ |

### 5.4 Phase D: Launch Remaining Apps

| Step | Key | Expected behavior | Pass? |
|------|-----|-------------------|-------|
| D1 | `` ` `` → select Linen → Enter | Linen surface appears | ☐ |
| D2 | (visual) | Phase 5 app dot changes to Linen green | ☐ |
| D3 | `` ` `` → select Quil → Enter | Quil surface appears | ☐ |
| D4 | (visual) | Phase 5 app dot changes to Quil mauve | ☐ |
| D5 | `` ` `` → select Bell → Enter | Bell surface appears | ☐ |
| D6 | (visual) | Bell ring shows events (may be empty list) | ☐ |
| D7 | `` ` `` → select Atlas → Enter | Atlas overlay toggles | ☐ |
| D8 | (visual) | Screen may dim or overlay may appear | ☐ |
| D9 | `F10` | Atlas overlay toggles again | ☐ |
| D10 | `` ` `` → select Collar → Enter | Collar surface appears | ☐ |
| D11 | (visual) | Collar grants visible (12 expected) | ☐ |
| D12 | `` ` `` → select Mesh → Enter | Mesh surface appears | ☐ |
| D13 | (visual) | Mesh frame/tab topology visible | ☐ |

### 5.5 Phase E: Atlas Tint/Accent

| Step | Key | Expected behavior | Pass? |
|------|-----|-------------------|-------|
| E1 | `F5`–`F9` (scene switch) | Active scene changes, workspace indicator moves | ☐ |
| E2 | (visual) | Phase 5 tint swatch color changes with scene accent | ☐ |
| E3 | `F10` (Atlas toggle) | Atlas overlay toggles | ☐ |
| E4 | (visual) | Phase 5 palette dot reflects palette open/close state | ☐ |

### 5.6 Phase F: Stress / Stability

| Step | Key | Expected behavior | Pass? |
|------|-----|-------------------|-------|
| F1 | Rapid `J`/`K` in palette | No freeze, no glitch | ☐ |
| F2 | Rapid `` ` `` toggle ×5 | Palette opens/closes cleanly | ☐ |
| F3 | Launch all 7 apps | All 7 visible simultaneously | ☐ |
| F4 | F5→F6→F7→F8→F9 scene cycle | No freeze, workspace indicators follow | ☐ |
| F5 | Leave idle 60s | Clock advances, no freeze, no panic | ☐ |

## 6. Failure Triage Table

| Symptom | Likely Cause | First Action |
|---------|-------------|-------------|
| **Black screen** (no Limine) | USB not detected / UEFI boot failed | Try legacy BIOS boot; check Ventoy partition scheme (GPT vs MBR) |
| **Black screen** (after Limine) | Kernel boot failure / framebuffer not handed off | Rebuild ISO; check `QEMU preflight PASS` |
| **No keyboard input** | USB XHCI driver init failed | Check if keyboard is USB 2.0 port vs 3.0; try different port |
| **Freeze on boot** | Deadlock in proof function | Rebuild without `SEXOS_*_PROOF=1` env vars and re-test |
| **Wrong resolution** | EDID / GOP framebuffer mismatch | Note exact resolution; compare with QEMU fallback 1280×800 |
| **SilkBar missing or garbled** | FB handoff failed or BAR_BG_BUF overflow | Check that FB_W ≤ 2560; SilkBar only supports y<51 |
| **#PF / #GP / panic** | Kernel fault | **STOP immediately.** Do NOT reboot and retry — record exact visible state (photo). Re-run QEMU preflight. If QEMU passes but hardware faults, create `docs/handoff/HARDWARE_FAULT_<date>.md` with: photo, exact ISO build, git commit, boot sequence, visible register dump if any. |
| **Reboot loop** | Triple fault or watchdog | **STOP.** Same as #PF/#GP — record, don't retry blindly. Create hardware-fault handoff. |
| **SilkBar indicators missing** | Phase 5 proof not enabled | Verify `SEXOS_SILKBAR_PHASE5_PIXEL_PROOF=1` in build. Re-run QEMU preflight. |
| **Palette missing rows** | Proof env vars missing | Verify all 41 env vars; check `grep "export SEXOS" scripts/run_daily_driver_proof.sh | wc -l` = 41. |
| **Slow / stuttering render** | QEMU vs hardware timing delta | Normal — hardware may be faster. Note if 60+ seconds with no response = freeze. |

### 6.1 Hardware Failure Protocol (DO NOT SKIP)

```
1. STOP touching the keyboard.
2. Take a photo of the screen.
3. Note exact: ISO build time, git commit hash, boot sequence steps.
4. Re-run QEMU preflight. If QEMU passes = hardware-specific issue.
5. Create docs/handoff/HARDWARE_BLOCKER_<date>_<symptom>.md
6. Do NOT edit source to "try things" on hardware.
7. Wait for root-cause analysis before next hardware boot.
```
| **SilkBar indicators missing** | Phase 5 proof not enabled | Verify `SEXOS_SILKBAR_PHASE5_PIXEL_PROOF=1` in build |
| **Palette missing rows** | Proof env vars missing | Verify all 41 env vars are exported during build |
| **Slow / stuttering render** | QEMU vs hardware timing delta | Normal — hardware may be faster. Note if 60+ seconds with no response = freeze. |

## 7. Hardware-Specific Notes

### Alienware
- **Boot key**: F12 for boot menu (tap repeatedly after power-on)
- **UEFI vs Legacy**: Try UEFI first; if Ventoy doesn't appear, enable Legacy Boot in BIOS
- **USB ports**: Rear USB 2.0 ports may be more reliable than front USB 3.0 for boot
- **Resolution**: Likely 1920×1080 or 2560×1440; SilkBar `BAR_BG_W_CAP` is 2560

### Ventoy
- **ISO placement**: Copy to root of Ventoy partition (not in subdirectory)
- **Partition scheme**: GPT for UEFI, MBR for Legacy
- **Secure Boot**: May need to disable in BIOS (Ventoy with Secure Boot requires extra setup)

## 8. Post-Boot Log Capture (if available)

If serial-over-USB or network logging is configured, capture and scan:

```bash
grep -E "fault.kill|#PF|#GP|panic|KERNEL PANIC|FATAL|sexdisplay.ready|silk.contract" hardware_boot.log
```

Without serial capture: phone video is the primary evidence. Note exact timestamps for each phase.

## 9. Success Criteria

| Criterion | Threshold |
|-----------|-----------|
| Boot to desktop | SilkBar + background gradient visible |
| Clock ticking | Clock advances (visible second changes or minute rollover) |
| Palette open/close | Backtick toggles palette overlay |
| All 7 apps launchable | Each app appears on Enter from palette |
| Phase 5 indicators | App dot, tint swatch, palette dot visible |
| No freeze (60s idle) | Clock continues, no visual stall |
| No kernel faults | No #PF, #GP, panic text, or spontaneous reboot |
| Scene switching | F5–F9 cycle workspaces |

## 10. QEMU Reference Values (for comparison)

| Metric | QEMU Value | Hardware Expected |
|--------|-----------|-------------------|
| Resolution | 1280×800 (fallback) | Native (likely 1920×1080) |
| Boot time to bar | ~2s | ~3–5s (slower disk I/O) |
| Clock delay before tick | ~1s | ~1s |
| SilkBar update rate | ~2 fps first 30s, then 1 fps | Similar or faster |
| Phase 5 draw markers | 8 (budgeted) | 8 (same budget) |
| Phase 3 receive counts | 7 (budgeted) | Varies (more focus changes possible) |

## Handoff Path

```
docs/handoff/REAL_HARDWARE_DAILY_DRIVER_BOOT_PROOF_V1.md  ← THIS DOCUMENT
docs/handoff/DAILY_DRIVER_PROOF_PROFILE_V1.md               ← proof profile reference
docs/handoff/DAILY_DRIVER_MASTER_GATE_V1.md                 ← gate reference
docs/handoff/SILKBAR_PHASE5_GATE_UPDATE_V1.md               ← latest gate update
```

