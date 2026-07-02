# Repo Layout

## Purpose
This document defines the top-level layout for SexOS so active code stays discoverable and legacy artifacts do not crowd the repository root.

## Top-Level Ownership
- `kernel/` kernel runtime, boot path, interrupts, scheduler, MMU/PKU logic.
- `servers/` userland PD servers.
- `apps/` user-facing apps/binaries.
- `crates/` shared Rust crates.
- `scripts/` automation and runbooks.
- `docs/` architecture, handoff, runbooks, and reference docs.
- `tools/` host-side helper tools.
- `tests/` automated tests.
- `archive/` historical artifacts preserved for later deletion.

## Root Policy
Root should contain only:
- project control files (`Cargo.toml`, `Cargo.lock`, `Makefile`, `rust-toolchain.toml`, `x86_64-sex.json`, `LICENSE`, etc.)
- boot config/assets that are intentionally root-level for current build flow (`limine.cfg`, selected EFI binaries)
- compatibility entrypoints (`dev.sh`, `build_payload.sh`)
- temporary compatibility shims that forward to `scripts/legacy/*`

New ad-hoc logs, backups, one-off scripts, and prompts should not be added to root.

## Script Organization
- `scripts/build/` canonical build + ISO generation paths.
- `scripts/run/` QEMU/runtime gates.
- `scripts/dev/` repo hygiene and utility checks.
- `scripts/legacy/` old one-off scripts retained for transition.

## Transitional Shims
Legacy root scripts may remain as tiny forwarding wrappers to `scripts/legacy/*` for compatibility.
These shims should be removed after downstream references are migrated.

## Enforcement
Use `scripts/dev/check_root_allowlist.sh` in report mode during cleanup and strict mode in CI later.
