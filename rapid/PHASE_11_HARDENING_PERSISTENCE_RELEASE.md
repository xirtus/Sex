# PHASE 11: Hardening + Persistence + Release

## Goal
Make SexOS a daily-driver OS. Persistence across reboots, crash resilience, installer for bare-metal deployment, accessibility basics, final audit, performance proof. This is the "ship it" phase.

## Ownership
- **sexstore** (exclusive): persistence layer, key-value + object storage, append-only log
- **sexboot** (exclusive): installer, live USB, bootloader integration, update rollback
- **silk-shell** (integration): session save/restore, power dialog, crash handler
- **Collar/security** (integration): secure boot attestation
- **All servers** (audit): final contract enforcement, no-POSIX audit, unsafe audit
- **Developer** (exclusive): documentation, performance benchmarks, release artifacts

## What Already Exists
- All core servers exist and boot (silk-shell, sexdisplay, silkbar, sexinput, sexusb, linen, mesh/collar, sexfiles, sexshop, bell, quil)
- Build system produces bootable ISO (`sexos-v1.0.0.iso`)
- ABI hash gate prevents silent contract drift
- Runtime proof markers validate all contracts at boot
- No persistence layer exists (settings lost on reboot)
- No installer exists (requires bare-metal boot)
- No crash reporter (panics = silent reset)
- No documentation beyond handoff docs

## Bundle (Subphases)

| Sub-phase | Task | Detail | Effort | Priority |
|-----------|------|--------|--------|----------|
| **11A: Harden** | Crash reporter | Unhandled panic → capture message → store in sexstore → display on next boot | 4h | High |
| **11A: Harden** | Final POSIX audit | grep for std/thread/alloc in all servers, eliminate any remaining | 2h | High |
| **11A: Harden** | Final framebuffer audit | Verify sexdisplay is sole framebuffer writer (static analysis) | 2h | High |
| **11A: Harden** | unsafe audit | Review all `unsafe` blocks in servers, document safety invariants | 4h | High |
| **11A: Harden** | Performance proof | Boot time, render latency, input latency benchmarks | 4h | Medium |
| **11A: Harden** | Documentation | Architecture docs, user guide, API reference (from handoffs) | 8h | Medium |
| **11B: Persist** | sexstore key-value | Persistent key-value store via append-only log on disk/block device | 8h | High |
| **11B: Persist** | Settings persistence | Save appearance tokens, input config, workspace layout across reboots | 4h | High |
| **11B: Persist** | Session save/restore | Save open windows, positions, tabs, focus state on shutdown → restore on boot | 6h | Medium |
| **11B: Persist** | sexstore object storage | Larger object storage (file data, notification history) | 4h | Low |
| **11C: Release** | Installer/live USB | Bootable installer ISO, partition setup, file copy | 8h | High |
| **11C: Release** | Update rollback | Versioned boot images, fallback on boot failure, atomic update | 6h | Medium |
| **11C: Release** | Power dialog | Shutdown, restart, suspend options via Quil panel | 2h | Medium |
| **11C: Release** | Accessibility basics | High contrast theme, larger targets, keyboard navigation audit | 4h | Medium |

## Smallest First Step
**Crash reporter**: Add a panic handler that writes the panic message to a fixed memory location before resetting. On next boot, check that location and display the message. This is trivially safe (fixed memory address, no allocation) and immediately useful for debugging.

## Dependencies
- **11A (Harden)**: Blocks on nothing — cores are all built, audit is purely analytical
- **11B (Persist)**: Blocks on sexstore block device driver (needs sexdrive or existing storage path)
- **11C (Release)**: Blocks on 11B (installer needs persistence to save install state)
- **Can parallelize**: All three subphases can proceed independently. Audit (11A) is pure analysis. Persistence (11B) is storage driver work. Release (11C) is bootloader/ISO work.

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| sexstore needs block device driver (AHCI/NVMe) | High | HIGH | Start with a simple RAM-backed store (persists within boot only). True disk persistence requires sexdrive (AHCI or virtio-blk). QEMU virtio-blk is simpler than real AHCI. |
| Installer needs partition manipulation | High | High | V1: Installer is "copy ISO to target disk" — no partition resizing. Target must be a dedicated disk. Warn user: "This will erase the target disk." |
| Update rollback needs bootloader support | Medium | High | V1: Keep last 2 boot images on disk. Bootloader menu allows selecting version. Only implement after basic installer works. |
| Session restore is complex (all server state) | High | Medium | V1: Save only surface IDs, positions, and active workspace. Don't try to restore app internal state (that's the app's responsibility). |
| Performance benchmarks show slow rendering | Medium | Low | Profile composite_pixel() and identify hotspots. Optimize only if benchmarks show >16ms frame time (below 60fps). |

## Exit Criteria (Done Checklist)

**Phase 11A (Harden):**
- [ ] Crash reporter captures panic messages and displays on next boot
- [ ] Zero POSIX/std/thread imports in all server code
- [ ] Zero framebuffer writes outside sexdisplay (verified by static analysis)
- [ ] All unsafe blocks documented with safety invariants
- [ ] Performance benchmarks: boot time <5s, render <16ms, input latency <10ms
- [ ] Architecture doc, user guide, API reference committed

**Phase 11B (Persist):**
- [ ] sexstore key-value store works (write key, reboot, read key returns value)
- [ ] Appearance token settings persist across reboot
- [ ] Workspace layout persists across reboot
- [ ] Session restore: open windows restored to previous positions
- [ ] sexstore object storage: write file, reboot, read file returns same content

**Phase 11C (Release):**
- [ ] Installer boots on bare-metal, copies OS to target disk, reboots into installed OS
- [ ] Live USB boots on any UEFI system (or BIOS with Limine)
- [ ] Update rollback: install version A, update to version B, rollback to version A
- [ ] Power dialog: shutdown and restart work from Quil panel
- [ ] Accessibility: high contrast theme, keyboard-navigable all dialogs

**Overall:**
- [ ] Build passes. Boot passes. No panic.
- [ ] All gates pass (gate_build, gate_boot, gate_markers, gate_no_std, gate_fb)

## Testing Strategy
- **Crash reporter**: Trigger intentional panic, verify message captured and displayed on next boot.
- **Persistence**: Set appearance preset to VioletGlass, reboot, verify preset is still VioletGlass.
- **Installer**: Boot installer ISO in QEMU with secondary virtio disk, install, reboot from installed disk, verify OS boots.
- **Session restore**: Open Quil + terminal, arrange windows, reboot, verify windows restored to same positions.
- **Performance**: `time` the boot process. Use QEMU -icount for deterministic frame timing.
- **Regression**: Phase 0 gates all pass. All proof markers fire at expected counts.

## Efficiency Opportunity
**Skip the installer for V1.** If the target audience is developers running in QEMU, a bootable ISO is sufficient. Bare-metal installation adds enormous complexity (partitioning, bootloader config, driver support) for marginal gain. A "QEMU-only" release is genuinely useful for 90% of the target audience.

**Skip update rollback for V1.** Versioned boot images and atomic updates require significant bootloader and storage infrastructure. Defer to V2. For V1, the release is the ISO — users rebuild from source.

**Focus Phase 11 on: crash reporter + persistence + documentation + audit.** These four deliverables make the OS reliable and usable. Installer, rollback, power dialog, and accessibility are polish that can ship incrementally after V1.

## Completeness Gain
Overall daily OS: **70–85% → 90–95%** (with crash reporter + persistence + docs). **90–95% → 100%** with installer + accessibility + all benchmarks.

## Files Changed
- `servers/sexstore/src/main.rs` (new PDX server — key-value + object storage)
- `servers/sexdrive/src/main.rs` (block device driver — AHCI or virtio-blk)
- `servers/silk-shell/src/main.rs` (session save/restore, crash handler, power dialog)
- `servers/quil/src/main.rs` (power dialog, installer UI, accessibility settings)
- `scripts/gate_all.sh` (includes all audit gates)
- `docs/ARCHITECTURE.md` (new — comprehensive architecture doc)
- `docs/USER_GUIDE.md` (new — user-facing documentation)
- `sexos_build_spec.toml` (release version bump)
- `scripts/entrypoint_build.sh` (release build variant)

## Forbidden
- Proprietary bootloader dependencies (use Limine — already in build system)
- Phone-home / telemetry (crash reporter is opt-in, local only)
- GPL-licensed components (LGPL exception for Limine is acceptable)
- Binary blob drivers
- Broad refactor (this is a stabilization phase, not a rewrite phase)
- Scope creep (every new feature belongs in V2 roadmap, not Phase 11)

## Next Phase
Release and daily-driver maintenance. No more phases — enter sustainment mode.

## Summary: 12 → 11 Phases

The original 12 phases were consolidated to 11 by merging Mesh (Phase 6) + Collar (Phase 7) into a single "System Graph + Authority" phase. Each remaining phase is bounded by ownership domain, has clear exit criteria, and can be parallelized where possible.

| Original | Revised | Consolidation |
|----------|---------|---------------|
| Phase 00 | Phase 00 | Unchanged |
| Phase 01 | Phase 01 | Unchanged |
| Phase 02 | Phase 02 | Unchanged |
| Phase 03 | Phase 03 | Unchanged |
| Phase 04 | Phase 04 | Unchanged |
| Phase 05 | Phase 05 | Unchanged |
| Phase 06 | Phase 06 | **Mesh + Collar merged** |
| Phase 07 | — | Merged into Phase 06 |
| Phase 08 | Phase 07 | Renumbered |
| Phase 09 | Phase 08 | Renumbered |
| Phase 10 | Phase 09 | Renumbered |
| Phase 11 | Phase 10 | Renumbered |
| Phase 12 | Phase 11 | Renumbered, reorganized |
