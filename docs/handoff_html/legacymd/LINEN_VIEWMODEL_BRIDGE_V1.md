# LINEN_VIEWMODEL_BRIDGE_V1

Date: 2026-05-07
Status: LANDED
Requires: LINEN_UI_STATIC_V1

## Files Changed

- `servers/silk-shell/src/main.rs` — 1 line

No Linen source changes. No kernel changes. No new opcodes. No sexdisplay changes.

## Ownership Map

| Resource | Owner | Notes |
|----------|-------|-------|
| Surface 200 (visual) | silk-shell | Creates via 0xEC at boot, is owner_pd |
| Surface 200 painter | silk-shell | 0xEF/0xFA/0xFB calls pass auth (owner) |
| Shell-local LINEN_OBJECTS[] | silk-shell | [Option<LinenObject>; 16], seed-initialized at boot |
| Linen PD SESSION objects | Linen PD | owner_pd = Linen kernel PD (7); not accessible to shell |

## Change

**Root cause**: Boot render at line 10143 called `linen_render_static_ui()` directly,
bypassing `linen_paint_surface()`. `linen_object_table_init()` is called at line 9994
(fills LINEN_OBJECTS with 6 seed objects before surface 200 creation). Count was 6 at
boot time, but explicit static call prevented the dispatcher from routing to
`linen_render_object_list()`.

**Fix**: line 10143:
```rust
// Before
unsafe { linen_render_static_ui(); }
// After
unsafe { linen_paint_surface(); }
```

`linen_paint_surface()` checks `linen_object_count()`. With 6 seed objects, routes to
`linen_render_object_list()`.

## Protocol Used

**Shell-local seed objects only.** No IPC to Linen PD.

`linen_object_table_init()` copies 6 entries from `LINEN_SEED_OBJECTS` const into
`LINEN_OBJECTS` static array. Seeds have `display_name: &'static str` — no struct change
needed. Render reads from shell-local table directly.

## pdx_call Return Shape

`pub fn pdx_call(slot, opcode, arg0, arg1, arg2) -> (u64, u64)` — tuple `(status, value)`.
Confirmed at `crates/sex-pdx/src/lib.rs:460`.

## Object Name Shape

Shell-side `LinenObject.display_name: &'static str` — static string literals in
`LINEN_SEED_OBJECTS`. No byte-array conversion needed for V1.

If live poll from Linen PD is added later: `SESSION.get(id, owner)` returns only first 8
bytes of name via `pdx_reply` (single u64). No offset arg exists in 0x43. Names > 8 chars
are truncated. Struct change to `name: [u8; 8]` or similar would be required at that point.

## STOP FIRST: Live Linen PD Poll Blocked

Shell cannot poll Linen PD's SESSION objects via 0x42 without Linen source changes.

**Root cause**: `SESSION.list(caller_pd, start_idx)` in `servers/linen/src/session.rs:118`
filters by `obj.owner_pd == caller_pd`. Linen creates its SESSION objects with
`owner_pd = Linen_kernel_PD (7)`. Shell's kernel-injected `caller_pd` is not 7.

The only bypass is `caller_pd == 0` (session.rs:122), but `caller_pd` is kernel-injected
and unforgeable — shell cannot pass 0.

**To unblock live poll**, one of:
1. Add new Linen opcode (e.g., OP_LINEN_LIST_ALL = 0x44) that calls `SESSION.list(0, idx)`
   internally — bypasses filter without exposing filter to caller. Requires Linen source edit.
2. Modify 0x42 handler to use arg1 as override owner_pd (trust-based, no auth). Linen edit.
3. Shell creates objects in Linen with `owner_pd = shell_pd` via OP_LINEN_CREATE — shell
   would only see its own objects; Linen's internal objects still filtered. Partial.

All options require Linen source changes. Deferred to LINEN_SEXFILES_LIST_V1.

## Visual Result

At boot with seed objects:
- Header: `LINEN_LIST_HEADER_COLOR` (teal-green, `SELECTED_LINEN_OBJECT_ID=0` fallback)
- List background: dark slate
- 5 accent bars: kind-specific colors (blue=Project, green=Document, amber=CodeFile,
  magenta=MediaAsset, brown=BuildArtifact)
- No selected row highlight (id=0 → no match → highlight skipped; safe)
- No text on surface (linen_render_object_list does not call 0xFB)

On first J/K: `linen_select_next_object()` sets SELECTED_LINEN_OBJECT_ID=1, repaint
shows selection highlight on row 0 (SexOS Kernel / Project kind, blue accent).

## Proof Markers

Boot:
```
[linen.object.seed] id=1 kind=0 name=SexOS Kernel
[linen.object.seed] id=2 kind=1 name=Compositor Lifecycle Spec
[linen.object.seed] id=3 kind=2 name=Silk Shell main.rs
[linen.object.seed] id=4 kind=3 name=Desktop Screenshot
[linen.object.seed] id=5 kind=4 name=Current ISO Build
[linen.object.seed] id=6 kind=5 name=Drafts
[linen.object_table.init] count=6
[linen.object_list.render] w=<W> h=<H>
[linen.object_list.row] id=1 kind=Project state=Loaded name=SexOS Kernel selected=false
...
[linen.row.reject] id=0 reason=not_found_in_visible_rows
[linen.object_list.done] count=6 rows=6
```

## Gap to LINEN_SEXFILES_LIST_V1

1. Linen PD SESSION objects unreachable from shell (owner filter — see STOP FIRST above)
2. Object names not rendered as text to surface (linen_render_object_list omits 0xFB)
3. Linen PD must populate its SESSION table from sexfiles before live data flows to shell
4. Shell struct change needed when live names are fetched (8-byte limit from 0x43)
