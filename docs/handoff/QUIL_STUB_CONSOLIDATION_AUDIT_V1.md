# QUIL_STUB_CONSOLIDATION_AUDIT_V1

**Status:** Complete — audit only, no code changes.
**Build:** ISO produced, no errors (commit `7070202`).
**Pre-requisite:** All Quil phases V1A–V1D committed.

---

## Summary

Audit confirming the Quil stub chain (V1A→V1D) is internally consistent and
ready to freeze before future editor protocol work. All 10 audit checks pass.
No contradictions found between code and handoff docs.

---

## Files Inspected

| File | Lines | Role |
|------|-------|------|
| `crates/sex-pdx/src/lib.rs` | 409 | ABI constants: slots + opcodes |
| `kernel/src/init.rs` | ~240 | Boot topology + cap grants |
| `servers/silk-shell/src/main.rs` | ~6600 | Shell policy owner, Quil lifecycle |
| `servers/quil/src/main.rs` | 59 | Quil PD server |
| `servers/sexdisplay/src/main.rs` | 1229 | Renderer-only (not inspected, zero Quil refs) |
| `docs/handoff/QUIL_SURFACE_STUB_V1A.md` | — | V1A handoff |
| `docs/handoff/QUIL_PD_SPAWN_V1B.md` | — | V1B handoff |
| `docs/handoff/QUIL_PROTOCOL_ASSIGN_V1C.md` | — | V1C handoff |
| `docs/handoff/QUIL_PD_ROUTE_PROOF_V1D.md` | — | V1D handoff |
| `docs/handoff/A7_DISPLAY_CONFORMANCE_V1.md` | — | Display boundary audit |
| `docs/handoff/A8_LIFECYCLE_PROOF_SCENARIOS_V1.md` | — | Lifecycle proof audit |

---

## Audit Checks

### Check 1: Quil PD boots as domain 9 ✅

```
kernel/src/init.rs:38 — module_paths[8] = "quil" → domain_id=9
kernel/src/init.rs:77 — quil_id set, [kernel.spawn.quil] marker
```

Domain 9 is the last spawn position, after sexstore (domain 8). No index
displacement of existing PDs.

---

### Check 2: SLOT_QUIL = 11 exists and is used only for shell→Quil ✅

```
crates/sex-pdx/src/lib.rs:355 — pub const SLOT_QUIL: u64 = 11;
```

Used in:
- `kernel/src/init.rs:160` — cap grant: shell→Quil (one-way)
- `servers/silk-shell/src/main.rs:9` — import
- `servers/silk-shell/src/main.rs:3436` — `pdx_call(SLOT_QUIL, OP_QUIL_PING, ...)`

**NOT** used in:
- `servers/quil/src/main.rs` — Quil does not import or use SLOT_QUIL

---

### Check 3: OP_QUIL_PING = 0xD0 exists and is handled by Quil ✅

```
crates/sex-pdx/src/lib.rs:101 — pub const OP_QUIL_PING: u64 = 0xD0;
```

Handled in:
- `servers/quil/src/main.rs` — match arm `OP_QUIL_PING` → `[quil.route.recv]`

Catch-all `_` arm in Quil yields safely (`[quil.unknown.yield]`).

---

### Check 4: Shell has cap to Quil ✅

```
kernel/src/init.rs:160:
    pd.grant_capability(sex_pdx::SLOT_QUIL, CapabilityData::Domain(quil_id));
```

Grant is one-way: silk-shell → Quil. No reverse grant exists.

---

### Check 5: Quil has no SLOT_DISPLAY/SHELL/STORAGE/INPUT/AUDIO/SILKBAR/SEXSTORE caps ✅

`rg -n "SLOT_" servers/quil/src/main.rs` returns **zero matches**.

Quil server only imports:
```rust
use sex_pdx::{pdx_listen_raw, serial_println, OP_QUIL_PING};
```

No `SLOT_*` constants imported. Quil only listens on default slot 0 (self
message ring). Zero kernel cap grants target Quil.

---

### Check 6: silk-shell remains owner of Quil surface lifecycle ✅

silk-shell owns all Quil frame/surface functions:
- `ensure_quil_frame()` — frame allocation in FRAMES slot
- `open_quil_in_active_scene()` — open/restore/placeholder
- `toggle_quil()` — minimize/restore lifecycle
- `focus_or_open_quil()` — focus management
- `quil_frame_id()` — frame lookup
- `SURFACE_ID_QUIL = 201` — constant defined in shell
- `QUIL_FRAME_ID = 3` — constant defined in shell

Quil server creates **no surfaces**, **no frames**, and calls **no 0xEC/0xEF**
opcodes.

---

### Check 7: sexdisplay remains renderer-only ✅

`rg -c "QUIL" servers/sexdisplay/src/main.rs` returns **zero matches**.

sexdisplay has no Quil awareness. It renders whatever surfaces the shell
instructs via 0xEC/0xEE/0xEF. Confirmed by A7 audit:

> "sexdisplay does not infer lifecycle state" — A7 Target 1
> "sexdisplay does not decide focus validity" — A7 Target 2
> "sexdisplay does not decide close/minimize/restore/destroy semantics" — A7 Target 3

---

### Check 8: No framebuffer writes outside sexdisplay ✅

Quil server explicitly confirms:
```
servers/quil/src/main.rs:21 — serial_println!("[quil.no_fb_write]");
```

Quil imports no pixel/framebuffer types, allocates no surface buffers, and
calls no display opcodes. Silk-shell only writes framebuffer through
`pdx_call(SLOT_DISPLAY, ...)` — sexdisplay is the sole framebuffer writer.

---

### Check 9: Unknown Quil messages yield safely ✅

```
servers/quil/src/main.rs:46-54 — catch-all _ arm:
    [quil.unknown.yield] — budget 8
```

Quil's `pdx_listen_raw(0)` internally yields when no message is available
(see sex-pdx:218-224). Unknown messages are logged and the loop continues.
No panic, no undefined behavior.

---

### Check 10: Handoff docs agree with committed code ✅

| Handoff | Commit | Agreement |
|---------|--------|-----------|
| `QUIL_SURFACE_STUB_V1A.md` | `7ad4609` | ✅ 3 markers present in code |
| `QUIL_PD_SPAWN_V1B.md` | `8e78fb5` / `f8b991d` | ✅ Domain 9, PDX loop, no caps |
| `QUIL_PROTOCOL_ASSIGN_V1C.md` | `0a5ca86` | ✅ SLOT_QUIL=11, OP_QUIL_PING=0xD0 |
| `QUIL_PD_ROUTE_PROOF_V1D.md` | `7070202` | ✅ All 4 file changes match |

No contradictions found.

---

## Constants/Caps Summary

| Constant | Value | Defined in | Used by |
|----------|-------|------------|---------|
| `SLOT_QUIL` | 11 | `crates/sex-pdx/src/lib.rs:355` | shell→Quil cap grant, shell ping call |
| `OP_QUIL_PING` | 0xD0 | `crates/sex-pdx/src/lib.rs:101` | shell caller, Quil receiver |
| `SURFACE_ID_QUIL` | 201 | `servers/silk-shell/src/main.rs:62` | shell lifecycle only |
| `QUIL_FRAME_ID` | 3 | `servers/silk-shell/src/main.rs` | shell FRAMES slot |
| — | Domain 9 | `kernel/src/init.rs` | PD spawn ID |

**Only one cap grant:** `silk-shell → SLOT_QUIL → quil_id` (one-way).

---

## Route Proof Confirmation

When `open_quil_in_active_scene()` runs (F9 open or restore):

```
1. ensure_quil_frame()          — allocates frame slot
2. restore_minimized_frame()?   — if minimized, restore
3. pdx_call(SLOT_DISPLAY, ...)  — set geometry
4. pdx_call(SLOT_DISPLAY, ...)  — fill rect (placeholder)
5. pdx_call(SLOT_QUIL, OP_QUIL_PING, ...)  → [shell.quil.route.ping]
                                       ↓
                              quil server receives:
                              → [quil.pdx.listen] msg
                              → [quil.route.recv]
6. snap_capture_layout()
```

Both markers (`[shell.quil.route.ping]` and `[quil.route.recv]`) are budgeted
at 8 hits each.

---

## Ownership Summary

```
Quil surface lifecycle OWNER: silk-shell
Quil frame OWNER:             silk-shell (FRAMES slot)
Quil placeholder OWNER:       silk-shell (via sexdisplay 0xEF)
Quil PD server:               listens on slot 0, handles OP_QUIL_PING only
Renderer:                     sexdisplay (sole framebuffer writer)
```

Quil PD has no authority over its own surface, frame, or placeholder. All
lifecycle policy is in the shell. This is the intended topology before
editor protocol — the shell retains full control.

---

## Contradictions Found

**None.** All audit checks pass. All handoff docs agree with committed code.
No stale references, no orphaned constants, no unguarded paths.

---

## Build Result

```sh
$ ./scripts/entrypoint_build.sh
[SEXOS ENTRYPOINT] success
ISO produced: sexos-v1.0.0.iso
Warnings: only pre-existing (unused import in sexstore, unnecessary unsafe blocks)
```

---

## Ready for Future Editor Protocol?

**Yes.** The Quil stub chain is internally consistent. Key pre-conditions for
editor protocol:

| Pre-condition | Status | Notes |
|---------------|--------|-------|
| Quil PD boots | ✅ | Domain 9, no caps |
| Shell→Quil route proven | ✅ | SLOT_QUIL + OP_QUIL_PING |
| Quil has no display caps | ✅ | Zero SLOT_* imports |
| Shell owns surface lifecycle | ✅ | Proven V1A markers |
| Sexdisplay boundary clean | ✅ | A7 conformance confirmed |
| Lifecycle FSM proven | ✅ | A8 audit, all transitions covered |
| Build green | ✅ | ISO produced |
| Handoff docs consistent | ✅ | All phases match code |

**No issues blocking future editor work.** When editor protocol is designed,
it will need deliberate cap grants (likely SLOT_DISPLAY for surface content,
possibly SLOT_INPUT for keyboard handling) and a protocol extension beyond
OP_QUIL_PING.

---

## Track D (Accessibility) Note

The user identified Track D (accessibility) as the next focus before
editor logic. This audit confirms Quil is safely frozen and won't interfere
with accessibility work. Key accessibility-relevant invariants:

- Quil has no keyboard input path (no SLOT_INPUT)
- Quil has no surface/framebuffer (no visual dependency)
- Shell retains full focus/lifecycle authority (accessibility narration
  can be added to shell without Quil conflicts)
- Quil's unknown-message yield is safe for diagnostic probing

---

## References

- `docs/handoff/QUIL_SURFACE_STUB_V1A.md` — shell-side lifecycle proof
- `docs/handoff/QUIL_PD_SPAWN_V1B.md` — PD boot (domain 9)
- `docs/handoff/QUIL_PROTOCOL_ASSIGN_V1C.md` — slot/opcode audit
- `docs/handoff/QUIL_PD_ROUTE_PROOF_V1D.md` — route proof implementation
- `docs/handoff/A7_DISPLAY_CONFORMANCE_V1.md` — display boundary
- `docs/handoff/A8_LIFECYCLE_PROOF_SCENARIOS_V1.md` — lifecycle proof
