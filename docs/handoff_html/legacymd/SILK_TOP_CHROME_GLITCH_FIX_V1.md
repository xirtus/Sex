# SILK_TOP_CHROME_GLITCH_FIX_V1

## Status: PATCHED — build clean

## Root Cause

**[silk.chrome.glitch.root]**

`close_surface_from_frame_light` (silk-shell `main.rs:14903`) decrements
`frame.tab_count` in the shell model but never calls `send_frame_tab_info`
for the surviving frame.

sexdisplay retains the pre-close `tab_count` (e.g. 2) in the surface slot
because `0xEC` upsert preserves `tab_count/active_tab/chrome_flags` for
existing active surfaces (sexdisplay `main.rs:2016`).

With stale `tab_count=2` and only 1 real tab, the tab strip renderer
(`composite_pixel`, sexdisplay `main.rs:343-360`) divides the chrome band
into 2 slots and renders `TAB_INACTIVE_COLOR` (0x0045475A) for the phantom
second slot — producing the visible dark glitch strip across the top chrome.

## Audit Trail

**[silk.chrome.glitch.audit]** — Files reviewed:
- `servers/sexdisplay/src/main.rs` — composite_pixel tab strip (lines 343-360), 0xEC upsert (2016), 0xFD handler (2352-2414)
- `servers/silk-shell/src/main.rs` — close_surface_from_frame_light (14903-15063), send_frame_tab_info (16802-16844), frame_tab_count (16682), tile_active_scene_frames (6972)

Key finding: `tile_active_scene_frames` sends `0xEC` to reposition the
surviving surface but does NOT call `send_frame_tab_info`. This is correct
for resize/move (chrome_flags preserved is intentional) but wrong after
tab removal (tab_count must be pushed).

## Fix

**[silk.chrome.glitch.fix]** — `servers/silk-shell/src/main.rs`

In `close_surface_from_frame_light`, after the FRAMES loop that removes the
closed tab and compacts remaining tabs:
- Added `surviving_frame_id: Option<u32>` captured when `frame.tab_count > 0` after close
- After the `frame_emptied` destruction block, call `send_frame_tab_info(fid)` for the surviving frame

This pushes the correct `tab_count` and `active_tab` to sexdisplay immediately
after tab removal, before `tile_active_scene_frames` repositions the surface.
sexdisplay then renders the correct number of tab slots on the next frame.

No change to sexdisplay, kernel, ABI, or proof markers.

## Markers Added

| Marker | Location | Meaning |
|--------|----------|---------|
| `[silk.chrome.glitch.audit]` | silk-shell:14983 | Audit comment at tab-removal site |
| `[silk.chrome.glitch.root]` | silk-shell:15029 | Comment marking surviving-frame capture |
| `[silk.chrome.glitch.fix]` | silk-shell:15050-15056 | Log when fix fires (send_frame_tab_info call) |
| `[silk.chrome.skip.invalid]` | sexdisplay:2396 | Logged when tab_count=0 received (no chrome) |
| `[silk.chrome.clear]` | sexdisplay:2398 | Logged when tab_count>0 received (chrome redrawn) |

## Files Changed

- `servers/silk-shell/src/main.rs` — close_surface_from_frame_light, ~15 lines added
- `servers/sexdisplay/src/main.rs` — 0xFD handler budget block, ~6 lines added (markers only)
- `servers/silk-shell/src/main.rs.bak_chrome_glitch_v1` — backup before patch

## Proof Commands

```
./scripts/entrypoint_build.sh   # must exit [SEXOS ENTRYPOINT] success
# Boot QEMU with serial log:
# grep log for:
#   - No #PF/#GP/panic/fault.kill
#   - [silk.frame.lights.render] or clock_visible_seconds
#   - [silk.chrome.glitch.fix] fires after each tab close
#   - [silk.chrome.clear] confirms sexdisplay received updated tab_count
#   - Visual: top chrome strip clean after closing a tab from multi-tab frame
```

Build result: `[SEXOS ENTRYPOINT] success` — no errors, warnings pre-existing only.

## Recurrence Prevention

This class of bug (shell model updated, sexdisplay not notified) can recur if:
- New tab-removal paths are added without `send_frame_tab_info` call
- Frame merge/split operations update tab_count without notifying display

**Rule:** Any code path that modifies `frame.tab_count` or `frame.active_tab`
must call `send_frame_tab_info(frame_id)` before returning if the frame survives.

## Scope

Single-domain fix (silk-shell chrome notification). No ABI change, no kernel
change, no top_strip_hash golden change (tab strip is below top strip), no
backing-buffer redesign.
