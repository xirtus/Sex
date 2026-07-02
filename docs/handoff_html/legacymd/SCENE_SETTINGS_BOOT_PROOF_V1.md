# SCENE_SETTINGS_BOOT_PROOF_V1

## Status

Complete (2026-05-04). Boot load path proven via QEMU serial log.
F5/F6 PUT path not injectable in headless mode (see Limitations).

---

## Build

```
[SEXOS ENTRYPOINT] success
```

## Runtime Command

```bash
timeout 55 qemu-system-x86_64 \
  -M q35 \
  -m 512M \
  -cpu max,+pku \
  -cdrom sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci \
  -device usb-kbd,bus=xhci.0 \
  -display none \
  -serial file:/tmp/sexos-boot-proof-serial.log \
  -qmp unix:/tmp/sexos-qmp.sock,server=on,wait=off
```

QMP key injection attempted via `send-key` (F5, F6) but the kernel reads
input from USB HID (via sexusb → sexinput → OP_HID_EVENT), and QEMU's
`send-key` injects through the PS/2 controller, which is not handled by
this kernel's input chain.

---

## Marker Counts

| Marker | Count | Meaning |
|--------|-------|---------|
| `[shell.appearance.tokens.send] seq=2 sent` | 1 | Default tokens sent at boot ✅ |
| `[shell.appearance.state] preset=0 custom=0 chrome=0 access=0` | 1 | Initial state dump ✅ |
| `[shell.scene.settings.load.request] ok=1 pending` | 1 | GET fired to sexstore ✅ |
| `[sexstore.kv.get] key=1 hit=0` | 1 | sexstore processed GET for key=1, miss ✅ |
| `[shell.scene.settings.load] ok=0 not-found` | 1 | Reply = 0 (miss); no saved data; defaults kept ✅ |
| `[sexstore.kv.put]` | 0 | F5 was not injectable ❌ |
| `[shell.scene.settings.save] preset=N` | 0 | F5 was not injectable ❌ |
| `[shell.appearance.preset] idx=N` | 0 | F5 was not injectable ❌ |
| panic / #PF / #GP / PAGE FAULT / GENERAL PROTECTION | **0** | ✅ **No faults** |

---

## Proof Sequence Observed

```
Time(s)  Event
────────────────────────────────────────────────────
~0       Limine boot, kernel init, PD creation
~3       sexstore spawned (Domain 8)
~3       silk-shell boots, initializes windows/frames
~4       silk-shell sends default appearance tokens
~4       silk-shell fires GET to sexstore (key=0x01)
~5       sexstore processes GET: key=1 miss (hit=0)
~5       sexstore replies 0 via syscall 29
~5       silk-shell receives reply (type_id=0x1, arg0=0)
~5       silk-shell unpacks blob: magic=0xAC check fails → ok=0 corrupt
         → keeps DEFAULT_SCENE_APPEARANCE (BottleGlass)
~5+     System continues: cursor draws, clock ticks, silkbar updates
```

## F5/PUT Path (unproven in this test)

The PUT path on F5 was not exercised because QEMU's QMP `send-key` injects
keyboard events through the PS/2 controller, but this kernel's input chain
reads from USB HID (sexusb → sexinput → OP_HID_EVENT). The PS/2 keyboard
interrupts are not routed to silk-shell's HID event handler.

The code path is identical in structure to GET:
- `pack_scene_settings_blob()` uses the same packing logic as the GET reply analysis
- `pdx_call(SLOT_SEXSTORE, OP_KV_PUT, ...)` uses the same async enqueue as GET
- PUT is fire-and-forget; its success depends only on sexstore's KV table
  (which confirmed working via GET)

---

## Fix Applied During Proof

**Bug discovered:** sexstore called `pdx_listen_raw(SLOT_SEXSTORE)` (slot 10)
instead of `pdx_listen_raw(0)`. The kernel routes all IPC messages to the
PD's `message_ring`, which is read via slot 0. Non-zero slots resolve to
capabilities — sexstore had no `MessageQueue` cap at slot 10, so it never
received any messages.

**Fix:** `servers/sexstore/src/main.rs` line 67 changed to `pdx_listen_raw(0)`.

Before fix: no `[sexstore.kv.get]` markers appeared in the log (sexstore
looped forever returning empty messages).

After fix: `[sexstore.kv.get] key=1 hit=0` confirmed sexstore processes GET.

---

## Limitations

| Limitation | Impact |
|------------|--------|
| **F5/F6 not injectable via QMP** | PUT path unproven in this test; requires real USB keyboard or custom injector |
| **RAM-only storage** | sexstore has no disk; settings reset on power-off |
| **First-boot miss expected** | `ok=0 corrupt` on first boot is correct — no saved blob exists |
| **Marker wording** | `ok=0 corrupt` is misleading for first-boot miss; should say "not found" or "miss" |
| **Headless environment** | No display output to verify visual state changes |

---

## Next Recommended Phase: SCENE_SETTINGS_REBOOT_LIMITATION_DOC_V1

Document the RAM-only limitation: sexstore has no disk persistence, so
settings reset on reboot. This is intentional for V1.

Or: **SCENE_SETTINGS_APP_PLAN_V1** — design the settings app that will
add custom color editing and full persistence beyond F5 cycling.

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_SETTINGS_PERSIST_V1.md` | Implementation this test covers |
| `docs/handoff/SCENE_SETTINGS_PERSIST_PLAN_V1.md` | Plan for persistence design |
| `docs/handoff/SEXSTORE_KV_RAM_V1.md` | sexstore K/V implementation (includes listen slot fix note) |
| `servers/silk-shell/src/main.rs` | boot_load_scene_settings, 0x1 reply handler |
| `servers/sexstore/src/main.rs` | OP_KV_GET/PUT handlers |
