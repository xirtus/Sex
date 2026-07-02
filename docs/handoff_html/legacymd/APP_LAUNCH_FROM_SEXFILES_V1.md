# APP_LAUNCH_FROM_SEXFILES_V1

## Status: CONTRACT_VALIDATED / BLOCKER: silk-shell lacks SLOT_STORAGE cap

- date: 2026-05-06
- gate: SEXOS_APP_LAUNCH_FROM_SEXFILES_PROOF=1
- result: ALL PROOF MARKERS PASS (GREEN_MASTER)

## Summary

Implemented a proof gate validating the full app-launch-from-SexFiles-manifest
pipeline. The proof demonstrates that when a manifest is serialized to canonical
bytes (the "stored in SexFiles" representation), deserialized back ("read from
SexFiles"), validated through `AppManifest::unpack`, and passed to the existing
`handle_app_surface_req` launch route, the surface is correctly created, focused,
and the shell retains full ownership of focus policy.

The actual PDX call to read manifest bytes from SexFiles (`SLOT_STORAGE`) is
absent because silk-shell (PD 3) is not granted `SLOT_STORAGE` capability at
kernel boot time (see `kernel/src/init.rs`). This is a single-line kernel
change; the contract pipeline is fully validated.

## Launch Route

```
SexFiles (SLOT_STORAGE)          silk-shell (SLOT_SHELL)
      │                                 │
      │  OP_RAMFS_OPEN("app.manifest")  │  ── NOT WIRED (no cap grant)
      │ ◄────────────────────────────── │
      │  handle (u64)                    │
      │ ──────────────────────────────► │
      │                                 │
      │  OP_RAMFS_READ(handle, 0, 24)   │  ── NOT WIRED (no cap grant)
      │ ◄────────────────────────────── │
      │  packed manifest bytes          │
      │ ──────────────────────────────► │
      │                                 │
                               AppManifest::unpack(sid, tid, arg2)
                                         │
                               AppCapabilityBits::validate()
                                         │
                               handle_app_surface_req()
                                         │
                               ┌─────────┼──────────┐
                               │         │          │
                          create frame  register   upsert on
                          + tab         lifecycle  sexdisplay (0xEC)
                                         │
                               collar_auto_grant_from_manifest()
                                         │
                               tile_active_scene_frames()
                               try_set_focus()
```

The proof simulates the "read from SexFiles" step by packing an `AppManifest`
to `(surface_id, title_id, arg2)` bytes and unpacking them. This is the
identical byte representation that would be read from a SexFiles RamFS file.

## Proof Markers

| Marker | Status | Serial Output |
|--------|--------|---------------|
| `[app.launch.sexfiles.proof.read_manifest]` | PASS ok=1 | sid=500, title_id=100, app_id=10, caps=0x2 |
| `[app.launch.sexfiles.proof.validate]` | PASS ok=1 | valid, malformed_reject, unknown_caps_reject all true |
| `[app.launch.sexfiles.proof.surface_register]` | PASS ok=1 | sid=504 accepted=true, state=Visible |
| `[app.launch.sexfiles.proof.focus]` | PASS ok=1 | sid=504 focused=true, actual_focus=504 |
| `[app.launch.sexfiles.proof.reject]` | PASS ok=1 | all 5 rejection sub-tests pass |

### Rejection sub-tests (Stage 4):
- **zero_sid**: surface_id=0 rejected ✅
- **reserved_bits**: non-zero reserved bits in arg2 rejected ✅
- **unknown_caps**: unknown capability bit 0x80 rejected ✅
- **duplicate**: re-registering surface 504 rejected ✅
- **reserved_range**: surface_id < 200 rejected ✅

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Added `APP_LAUNCH_FROM_SEXFILES_PROOF` gate + 6-stage proof + dynamic surface fallback in `surface_is_alive` + `is_focusable_surface` |
| `docs/handoff/APP_LAUNCH_FROM_SEXFILES_V1.md` | This handoff document |

### Companion changes (enabling):
| `servers/silk-shell/src/main.rs:surface_is_alive` | Added lifecycle-table fallback for dynamic surfaces (sid >= 200) |
| `servers/silk-shell/src/main.rs:is_focusable_surface` | Added dynamic surface focusability via `surface_is_alive` check |

These companion changes fix pre-existing limitations where dynamically-registered
app surfaces (created via `handle_app_surface_req`) were invisible to
`surface_is_alive` and non-focusable — affecting ALL dynamic surfaces, not just
the proof. They are minimal, bounded, and only activate for `sid >= 200`.

## Files NOT Changed

| File | Reason |
|------|--------|
| `kernel/src/init.rs` | STOP FIRST: silk-shell needs SLOT_STORAGE cap grant for real SexFiles reads |
| `crates/sex-pdx/src/lib.rs` | STOP FIRST: ABI change not needed (existing RamFS opcodes suffice) |
| `servers/sexfiles/` | No changes needed — existing RamFS API is correct |
| `apps/sexdisplay/` | No changes needed — surface ops (0xEC, 0xED, 0xEE) unchanged |

## Build/Runtime Result

```bash
# Build with proof compiled in
SEXOS_APP_LAUNCH_FROM_SEXFILES_PROOF=1 ./scripts/entrypoint_build.sh

# Run gate
./scripts/master_runtime_gate.sh --skip-build --probe 25 --keep-log
```

Result: **GREEN_MASTER** — all 6 gates PASS.

## Remaining App-Launch Blockers

1. **silk-shell lacks SLOT_STORAGE cap** — kernel `init.rs` does not grant
   `SLOT_STORAGE` to PD 3 (silk-shell). Adding one capability grant line would
   enable the shell to read manifests from SexFiles. This is the single
   remaining blocker for the real route.

2. **Manifest storage format** — the canonical byte format for app manifests
   in SexFiles is `(surface_id: u64, title_id: u64, arg2: u64)` = 24 bytes,
   matching `AppManifest::pack()`. A file naming convention (e.g.,
   `"app.{app_id:04x}"`) is needed but not yet defined.

3. **Manifest creation tool** — no tool exists to write manifests to SexFiles.
   A future `app-pack` tool or manifest creation via Linen/Quil would be needed.

4. **Manifest versioning** — `AppManifest::VERSION = 0` for V1. Future versions
   need a migration path.

## Smallest Future Patch

When silk-shell is granted `SLOT_STORAGE` capability, the minimal wiring
would look like:

```rust
// In silk-shell/src/main.rs (NOT IMPLEMENTED — requires kernel cap grant)

fn launch_from_sexfiles(name: &[u8], caller_pd: u32) -> bool {
    // 1. Open manifest file from SexFiles
    let (n0, n1, n2_flags) = pack_name_for_ramfs(name);
    let (status, _) = pdx_call(SLOT_STORAGE, OP_RAMFS_OPEN, n0, n1, n2_flags);
    if status != 0 { return false; }
    let handle = status;

    // 2. Read 24 bytes (3 × u64) of manifest data
    // (requires multiple OP_RAMFS_READ calls, 8 bytes each)
    let mut buf = [0u64; 3];
    for i in 0..3 {
        let (status, _) = pdx_call(SLOT_STORAGE, OP_RAMFS_READ, handle, (i*8) as u64, 8);
        if status < 0 { return false; }
        buf[i] = status;
    }

    // 3. Close
    pdx_call(SLOT_STORAGE, OP_RAMFS_CLOSE, handle, 0, 0);

    // 4. Validate and launch
    let manifest = match AppManifest::unpack(buf[0], buf[1], buf[2]) {
        Ok(m) => m,
        Err(()) => return false,
    };
    handle_app_surface_req(manifest.surface_id, manifest.title_id, buf[2], caller_pd)
}
```

The proof already validates that `AppManifest::unpack()` + `handle_app_surface_req()`
work correctly. Only the PDX read step is missing.

## Contract Boundaries Preserved

- **No Linux/POSIX assumptions**: manifest is bounded bytes, not file paths
- **No std/libc/threads**: all I/O through PDX calls
- **MPK/PKU/PKEY isolation preserved**: SexFiles and silk-shell in separate PDs
- **sexdisplay sole framebuffer writer**: apps never get raw FB access
- **Shell owns focus/policy**: `try_set_focus`, Collar grants, lifecycle all
  remain shell-authoritative
- **No kernel edits in this scope**: kernel SLOT_STORAGE grant is a separate
  prerequisite
- **No sex-pdx ABI edits**: all opcodes are existing RamFS constants, locally
  mirrored
- **No new loader architecture**: reuses existing `handle_app_surface_req` route
- **No broad refactor**: `surface_is_alive` and `is_focusable_surface` changes
  are minimal fallback additions

## Gate Run Command

```bash
# Full (build + run)
SEXOS_APP_LAUNCH_FROM_SEXFILES_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log

# Skip-build (ISO pre-built with proof)
./scripts/master_runtime_gate.sh --skip-build --probe 25 --keep-log
```
