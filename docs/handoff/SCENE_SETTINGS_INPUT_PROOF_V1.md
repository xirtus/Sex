# SCENE_SETTINGS_INPUT_PROOF_V1

## Status

Complete (2026-05-04). F5/F6 scene settings input path proven via gated
synthetic keyboard proof in sexinput. All expected markers present in
serial log. Zero faults.

---

## Build

```
[SEXOS ENTRYPOINT] success
```

Built with `SEXOS_KEYBOARD_PROOF=1` to enable the gated synthetic proof.

## Runtime Command

```bash
timeout 60 qemu-system-x86_64 \
  -M q35 \
  -m 512M \
  -cpu max,+pku \
  -cdrom sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci \
  -device usb-kbd,bus=xhci.0 \
  -display none \
  -serial file:/tmp/sexos-kbd-proof.log
```

---

## Marker Counts

| Marker | Count | Meaning |
|--------|-------|---------|
| `[sexinput.kbd_proof.f5.down]` | 1 | F5 press at tick 50 ✅ |
| `[sexinput.kbd_proof.f5.up]` | 1 | F5 release at tick 55 ✅ |
| `[sexinput.kbd_proof.f6.down]` | 1 | F6 press at tick 150 ✅ |
| `[sexinput.kbd_proof.f6.up]` | 1 | F6 release at tick 155 ✅ |
| `[shell.appearance.preset]` | 2 | Two F5 cycles: idx=1, idx=2 ✅ |
| `[shell.scene.settings.save]` | 2 | Two saves (one per F5) ✅ |
| `[sexstore.kv.put]` | 2 | Two PUTs received by sexstore ✅ |
| `[shell.appearance.custom]` | 1 | One F6 tint cycle ✅ |
| `[shell.scene.settings.load.request]` | 1 | Boot GET fired ✅ |
| `[shell.scene.settings.load]` | 1 | Boot GET completed (not-found) ✅ |
| `[sexstore.kv.get]` | 1 | Boot GET processed by sexstore ✅ |
| panic / #PF / #GP | **0** | ✅ **No faults** |

---

## Proof Timeline

```
Line  Event
─────────────────────────────────────────────────────────
641   [shell.scene.settings.load.request] ok=1 pending    ← Boot GET fired
650   [sexinput.kbd_proof.f5.down]                       ← F5 press (tick 50)
651   [sexinput.kbd_proof.f5.up]                         ← F5 release (tick 55)
652   [sexinput.kbd_proof.f6.down]                       ← F6 press (tick 150)
653   [sexinput.kbd_proof.f6.up]                         ← F6 release (tick 155)
804   [sexstore.kv.get] key=1 hit=0                      ← Boot GET reply (miss)
824   [shell.scene.settings.load] ok=0 not-found          ← No saved blob; defaults kept
825   [shell.appearance.preset] idx=1                     ← F5 cycle 0→1
826   [shell.scene.settings.save] preset=1                ← Save preset=1
827   [shell.appearance.preset] idx=2                     ← Second F5 cycle 1→2
828   [shell.scene.settings.save] preset=2                ← Save preset=2
829   [shell.appearance.custom] mode=tint tint=1          ← F6 tint cycle (NO save)
989   [sexstore.kv.put] key=1 ok=1                        ← PUT for preset=1
990   [sexstore.kv.put] key=1 ok=1                        ← PUT for preset=2
```

### Key Confirmations

1. **F5 → preset cycle + persist**: Two F5 events produce two
   `[shell.appearance.preset]` + `[shell.scene.settings.save]` +
   `[sexstore.kv.put]` marker sets ✅
2. **F6 → tint only, no save**: One F6 produces `[shell.appearance.custom]
   mode=tint tint=1` but NO additional `[shell.scene.settings.save]` (count
   stays at 2) ✅
3. **Boot GET path still works**: `load.request` → `kv.get` → `load ok=0
   not-found` — same sequence as SCENE_SETTINGS_BOOT_PROOF_V1 ✅
4. **No faults**: Zero panic/#PF/#GP markers across the entire 60-second
   run ✅

---

## Changed Files

| File | Change |
|------|--------|
| `servers/sexinput/src/main.rs` | Added `KEYBOARD_PROOF_ENABLED` const (env-var gated); added `kbd_proof_stage` local variable; added synthetic keyboard proof block with F5/F6 sequence (ticks 50-155); added `[sexinput.kbd_proof.*]` markers |

### NOT modified

- `kernel/` — no kernel changes
- `servers/sexusb/` — no change
- `servers/silk-shell/` — no change
- `servers/sexstore/` — no change
- `crates/sex-pdx/` — no ABI hash change

---

## Proof Gate

```rust
/// Enables one-shot synthetic keyboard proof for F5/F6 HID event path.
/// Set env var `SEXOS_KEYBOARD_PROOF=1` at build time to enable.
/// Default (unset): no behavior change.
/// Only affects sexinput; no kernel changes.
const KEYBOARD_PROOF_ENABLED: bool = option_env!("SEXOS_KEYBOARD_PROOF").is_some();
```

Follows the same `option_env!` pattern as `KEYBOARD_CURSOR_ENABLED` (line 39).
Default (no env var): zero overhead, no behavior change.

---

## Proof Sequence

| Stage | Tick | Action | Marker |
|-------|------|--------|--------|
| 0 | 50 | F5 press (scancode 0x3F=63) via `pdx_call(SLOT_SHELL, OP_HID_EVENT, 63, 1, EV_KEY)` | `[sexinput.kbd_proof.f5.down]` |
| 1 | 55 | F5 release (scancode 0x3F=63, value=0) | `[sexinput.kbd_proof.f5.up]` |
| 2 | 100 | Second F5 press (no marker — silent) | — |
| 3 | 105 | Second F5 release (no marker — silent) | — |
| 4 | 150 | F6 press (scancode 0x40=64) | `[sexinput.kbd_proof.f6.down]` |
| 5 | 155 | F6 release | `[sexinput.kbd_proof.f6.up]` |
| 6 | — | Done — no replay | — |

The second F5 (stages 2-3) intentionally omits markers to verify that the
shell processes the event correctly even without sexinput marker confirmation.
The shell's `[shell.appearance.preset] idx=2` and `[shell.scene.settings.save]
preset=2` markers confirm the second F5 was received and processed.

---

## Verification Commands

```bash
# Verify all expected markers
grep -n "sexinput.kbd_proof" /tmp/sexos-kbd-proof.log
grep -n "appearance.preset" /tmp/sexos-kbd-proof.log
grep -n "settings.save" /tmp/sexos-kbd-proof.log
grep -n "appearance.custom" /tmp/sexos-kbd-proof.log
grep -c "sexstore.kv.put" /tmp/sexos-kbd-proof.log

# Confirm F6 does NOT cause save (should be exactly 2 from two F5s)
grep -c "settings.save" /tmp/sexos-kbd-proof.log   # expected: 2

# Confirm F6 tint cycle fired
grep -c "appearance.custom" /tmp/sexos-kbd-proof.log   # expected: 1

# Verify zero faults
grep -cE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/sexos-kbd-proof.log   # expected: 0
```

---

## Limitations

| Limitation | Impact |
|------------|--------|
| **Synthetic proof, not real keyboard** | Proves the HID event pipeline works but does not exercise real USB HID keyboard or PS/2 interrupt path |
| **No display output** | Headless QEMU; visual state changes (tint, preset) not visually confirmed |
| **RAM-only storage** | sexstore has no disk; settings reset on power-off |

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_SETTINGS_INPUT_PROOF_PLAN_V1.md` | Plan this phase implements |
| `docs/handoff/SCENE_SETTINGS_BOOT_PROOF_V1.md` | Previous proof — GET path only |
| `docs/handoff/SCENE_SETTINGS_PERSIST_V1.md` | Persistence implementation under test |
| `docs/handoff/SEXSTORE_KV_RAM_V1.md` | sexstore K/V implementation |
| `servers/sexinput/src/main.rs` | Target file — synthetic proof added |
| `servers/silk-shell/src/main.rs` | HID event handler, preset cycle, persist |
| `servers/sexstore/src/main.rs` | KV PUT/GET handlers |
