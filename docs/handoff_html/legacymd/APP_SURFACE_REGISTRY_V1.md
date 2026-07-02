# APP_SURFACE_REGISTRY_V1

**Status:** Active  
**Purpose:** Replace comment-only app surface registry with a tiny compile-time shell registry for OS-managed app surfaces.  
**Scope:** `servers/silk-shell/src/main.rs` only. No kernel/ABI/sexdisplay changes.  
**Prerequisites:** `LINEN_SURFACE_CONTROL_V1`, `QUIL_SURFACE_STUB_V1`, `SURFACE_ACTION_BINDINGS_V1`

---

## What Was Done

Added a compile-time `AppSurfaceSpec` registry for the two OS-managed app surfaces (Linen and Quil) with boot-time duplicate validation. Match arms for `surface_is_alive` / `is_focusable_surface` / `is_closeable_surface` remain hardcoded — the registry is documentation + validation, not a dynamic dispatch table.

### Registry Design

```rust
struct AppSurfaceSpec {
    surface_id: u64,
    frame_id: u32,
    name: &'static str,
    boot_x: i32,
    boot_y: i32,
    boot_w: u32,
    boot_h: u32,
    closeable: bool,
    focusable: bool,
}

const APP_SURFACES: [AppSurfaceSpec; 2] = [
    AppSurfaceSpec { surface_id: SURFACE_ID_LINEN, frame_id: LINEN_FRAME_ID, name: "linen", ... },
    AppSurfaceSpec { surface_id: SURFACE_ID_QUIL,  frame_id: QUIL_FRAME_ID,  name: "quil",  ... },
];
```

### Duplicate Detection

`app_surface_registry_validate()` runs at boot (after `snap_capture_layout()`, before `SVC_STATE_LISTENING`). It checks:

- **Duplicate surface_id**: If two entries share the same surface_id, logs `[shell.app_registry.duplicate] surface_id=N entries=i,j`
- **Duplicate frame_id**: If two entries share the same frame_id, logs `[shell.app_registry.duplicate] frame_id=N entries=i,j`
- **Success**: Logs `[shell.app_registry.valid] count=2`
- **Failure**: Logs `[shell.app_registry.error]` — shell continues safely (duplicate check is advisory, not a panic)

### Lookup Helpers (Optional)

```rust
fn app_surface_spec(surface_id: u64) -> Option<&'static AppSurfaceSpec>
fn app_surface_spec_by_frame(frame_id: u32) -> Option<&'static AppSurfaceSpec>
```

Marked `#[allow(dead_code)]` — available for future wiring but not called in V1.

### Changes

| Location | Change |
|----------|--------|
| After surface ID constants (line 84) | Added `AppSurfaceSpec` struct, `APP_SURFACES` const array |
| After array (line 126) | Added `app_surface_registry_validate()` with O(n²) duplicate scan |
| After validate (line 147) | Added `app_surface_spec()` and `app_surface_spec_by_frame()` (dead_code allowed) |
| Boot init (line 3673) | Added `app_surface_registry_validate()` call after snap_capture_layout |

### Existing Comment-Only Registry (preserved)

The original OS-owned surface ID registry comment (lines 68-77) is preserved as documentation for ALL surfaces (including panels, cursor, legacy app surfaces). The new compile-time `APP_SURFACES` covers only the two frame-owned app surfaces.

---

## Build Result

```
RUSTFLAGS="-C relocation-model=pic -C link-arg=-pie" cargo build \
    -Z build-std=core,compiler_builtins,alloc \
    -Z build-std-features=compiler-builtins-mem \
    --manifest-path servers/silk-shell/Cargo.toml \
    --target /home/xirtus_arch/x86_64-sex.json \
    --release

Finished release profile [optimized] in 0.87s
Warnings: 202 bin + 1 lib (all pre-existing)
Errors: 0
```

## Files Changed

- `servers/silk-shell/src/main.rs` (+65 lines: struct, const array, validate function, lookup helpers, boot call)

## Not In Scope (future)

- No dynamic/allocated registry
- No refactor of Linen/Quil helpers into generic framework
- No changes to match arms in `surface_is_alive`, `is_focusable_surface`, `is_closeable_surface`
- No changes to surface ID or frame ID constants
- No kernel/ABI/sexdisplay edits

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Add compile-time app surface registry with boot validation | APP_SURFACE_REGISTRY_V1 |
