# Bell Presence Marker Noise Cleanup V1

## Status: Merged

## Changes

### Problem
After proving the end-to-end Bell presence route, the following high-frequency
success-path markers were producing noise every ~2 seconds indefinitely:
- `[silkbar.bell.poll] sent` — every poll cycle
- `[bell.readcap.allow]` — every successful LIST call
- `[bell.list.recv]` — every LIST call
- `[bell.list.empty]` — every poll when no events
- `[bell.list.done]` — every poll when events exist

These markers served their proof purpose but waste budget on an idle system.

### Solution

#### Removed markers
| Marker | Reason |
|--------|--------|
| `[silkbar.bell.poll]` | High-frequency success-path noise |
| `[bell.readcap.allow]` | Redundant with reply marker |
| `[bell.list.recv]` | Redundant with reply marker |
| `[bell.list.empty]` | High-frequency noise when no events |
| `[bell.list.done]` | Redundant with reply marker |

#### Budget-reduced markers
| Marker | Old budget | New budget | Reason |
|--------|-----------|-----------|--------|
| `[bell.list.item]` | 16 | 8 | Event debugging (only fires when events exist) |

#### Kept markers (unchanged)
| Marker | Budget | Reason |
|--------|--------|--------|
| `[bell.list.reject]` (invalid_lane) | 4 | Critical protocol error |
| `[bell.list.reject]` (invalid_count) | 4 | Critical protocol error |
| `[bell.readcap.deny]` | 8 | Critical auth error |
| `[bell.list.redact]` | 8 | Privacy-critical |
| `[bell.list.reply]` | 8 | Task-prescribed success marker |
| `[silkbar.bell.poll.reply]` | 8 | Task-prescribed success marker |
| `[silkbar.bell.reject]` | 8 | Critical error marker |
| `[sexdisplay.bell.render]` | 8 | Task-prescribed success marker |

### Files Changed
- `servers/silkbar/src/main.rs` — removed `[silkbar.bell.poll]` marker block
- `servers/sexbell/src/main.rs` — removed `[bell.readcap.allow]`, `[bell.list.recv]`,
  `[bell.list.empty]`, `[bell.list.done]` markers; reduced `[bell.list.item]` budget 16→8
- `servers/sexdisplay/src/main.rs` — unchanged (already budgeted 8)

### Acceptance
- `./scripts/entrypoint_build.sh` passes
- End-to-end Bell presence works (verified via QEMU)
- No unbudgeted high-frequency success markers remain
- Reject/privacy markers remain metadata-only

### Runtime marker frequency (idle, no events)
```
[bell.list.reply]        ~every 2s (budget 8 → silent after 8)
[silkbar.bell.poll.reply] ~every 2s (budget 8 → silent after 8)
[sexdisplay.bell.render]  ~every 2s (budget 8 → silent after 8)
```
