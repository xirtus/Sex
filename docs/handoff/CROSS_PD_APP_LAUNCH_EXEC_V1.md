# CROSS_PD_APP_LAUNCH_EXEC_V1 — STOP FIRST

## Verdict: STOP FIRST — No safe route exists.

## 1. Current Spindle Slots
| Slot | Name | Purpose |
|------|------|---------|
| 5 | SLOT_DISPLAY | framebuffer/window ops |
| 10 | SLOT_STORAGE | SexFiles RamFS |
| 12 | SLOT_BELL | notification bridge |
| 8 | SLOT_LINEN | Linen object ops (0x41-0x47) |

**Missing**: SLOT_SHELL — no capability to communicate with silk-shell launcher.

## 2. Spindle's Own Documentation
```
"App launch      kernel spawn + SLOT_SHELL needed"  (line 817)
"Spindle cannot cross-PD spawn — use silk-shell."    (launch command)
```

## 3. Potential Routes Audited

| Route | Available? | Blocker |
|-------|-----------|---------|
| Direct (SLOT_SHELL) | ❌ | No kernel capability grant to Spindle's PD |
| Bell bridge | ❌ (partial) | Spindle can send Bell events but silk-shell has no launch-from-bell dispatch |
| Linen bridge | ❌ | Linen has no app-launch forwarding |
| Storage bridge | ❌ | Polling storage for launch requests violates "no blocking/ polling" |
| SLOT_SPINDLE reverse | ❌ | SLOT_SPINDLE (14) is HID input TO Spindle, unidirectional |

## 4. Exact Blockers
1. **Kernel capability**: Spindle's PD needs SLOT_SHELL grant (kernel init.rs change)
2. **Launch protocol**: No opcode in any existing slot for launch-intent
3. **Silk-shell handler**: No dispatch for receiving external launch requests

## 5. Phased Plan (Future)

### Phase 1: Bell Bridge (no kernel change)
- Spindle sends Bell event with new category=Launch (0x02), source=app_name
- Bell server forwards to subscribers (existing event pub/sub)
- Silk-shell subscribes to Bell events, adds launch-from-bell dispatch
- **Risk**: Requires Bell server to accept new category + silk-shell subscription
- **ABI**: Bell category table extension (SexFiles-local, not kernel/pdx)

### Phase 2: SLOT_SHELL Grant (kernel change)
- Kernel init.rs adds SLOT_SHELL capability to Spindle's PD
- Collar policy table updated to allow the grant
- Spindle sends launch-intent directly to silk-shell via new opcode
- **Risk**: Kernel capability grant requires kernel edit ❌ STOP FIRST

## 6. Decision
**STOP FIRST** — Documented.  No source changes.
Phase 1 (Bell bridge) is the safest path forward when app protocol changes are permitted.
Phase 2 requires kernel edit — only when real native app spawn is needed.
