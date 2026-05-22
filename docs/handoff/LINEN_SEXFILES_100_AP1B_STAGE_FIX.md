# LINEN_SEXFILES_100_AP1B_STAGE_FIX

## Date
2026-05-22

## A) Files Changed

1. `sexos_build_spec.toml` — Removed `rustflags = "--cfg linen_diskfs_slot_proof"` from the `build_linen` [[stage]].
2. `scripts/entrypoint_build.sh` — Added post-staging verification gate (step 4) that checks staged linen carries `linen.sexfiles100.audit.begin`.
3. `docs/handoff/LINEN_SEXFILES_100_AP1B_STAGE_FIX.md` — This document.

## B) Exact Root Cause

The entrypoint build spec (`sexos_build_spec.toml`) passed `--cfg linen_diskfs_slot_proof` in the `rustflags` for the `build_linen` stage. This set `LINEN_DISKFS_SLOT_PROOF_ENABLED = true` at compile time via `cfg!()` in `servers/linen/src/main.rs` line 131-132.

In `_start()`, the call to `linen_init_session()` — which contains both `[linen.sexfiles100.audit.begin]` and `[linen.sexfiles100.audit.done]` markers — is gated behind `!LINEN_DISKFS_SLOT_PROOF_ENABLED` (line 618). When the slot proof cfg is active, this condition evaluates to `false`, making `linen_init_session()` dead code. The compiler eliminates the entire function (including its string literals) during release optimization.

Result: the source contained the markers but the compiled binary did not.

## C) Minimal Diff Summary

### sexos_build_spec.toml

```diff
 [[stage]]
 id = "build_linen"
 action = "cargo_manifest"
 manifest = "servers/linen/Cargo.toml"
 source_artifact = "target/x86_64-sex/release/linen"
 dest_artifact = "iso_root/servers/linen"
-rustflags = "--cfg linen_diskfs_slot_proof"
```

### scripts/entrypoint_build.sh

Added after `./scripts/sexos_build_trace.sh "$SPEC_PATH"`:

```diff
+ # 4) Build-time verification: staged linen must carry sexfiles100 audit markers
+ # NOTE: grep without -q (with >/dev/null) avoids SIGPIPE on strings under pipefail
+ if ! strings iso_root/servers/linen | grep "linen.sexfiles100.audit.begin" > /dev/null; then
+   fail "[SEXOS ENTRYPOINT] ERROR: staged linen missing sexfiles100 markers"
+ fi
+ echo "[SEXOS ENTRYPOINT] verification: staged linen markers present"
```

### Why grep without `-q`

The entrypoint script uses `set -euo pipefail`. With `pipefail`, `grep -q` causes `strings` to receive SIGPIPE (exit 141) when grep exits early after finding a match. The pipefail option propagates this 141 as the pipeline exit code, causing `set -e` to abort before `!` can negate. Using plain `grep` (which reads all input before exiting, unlike `-q`) avoids SIGPIPE entirely.

## D) Proof Commands and Results

```bash
# Syntax checks
$ bash -n scripts/entrypoint_build.sh
# (no output = pass)

$ bash -n build_payload.sh
# (no output = pass)

# Full build
$ ./scripts/entrypoint_build.sh
# ...
# [SEXOS TRACE] deterministic sequence complete
# [SEXOS ENTRYPOINT] verification: staged linen markers present
# [SEXOS ENTRYPOINT] success

# Marker verification
$ strings iso_root/servers/linen | grep "linen.sexfiles100"
# [linen.sexfiles100.audit.begin][linen.sexfiles.list.begin]...
# *[linen.sexfiles100.audit.done] ok=1 count=
```

Both `linen.sexfiles100.audit.begin` and `linen.sexfiles100.audit.done` are confirmed present in the staged binary.

## E) Commit Commands

```bash
git add sexos_build_spec.toml scripts/entrypoint_build.sh docs/handoff/LINEN_SEXFILES_100_AP1B_STAGE_FIX.md
git commit -m "linen: fix entrypoint staging — remove slot proof cfg gate, add marker verification

Root cause: --cfg linen_diskfs_slot_proof in build spec made
linen_init_session() dead-code at compile time, eliminating
SexFiles 100 audit markers from the binary.

Fix:
- Remove rustflags=--cfg linen_diskfs_slot_proof from
  sexos_build_spec.toml build_linen stage
- Add post-staging strings verification in entrypoint_build.sh
  (grep without -q to avoid SIGPIPE under pipefail)

Proof: strings iso_root/servers/linen | grep linen.sexfiles100
shows both .audit.begin and .audit.done markers present."
```
