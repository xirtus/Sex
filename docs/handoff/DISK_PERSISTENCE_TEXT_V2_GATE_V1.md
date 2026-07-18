# Disk Persistence + Text-Model V2 — Sprint Closeout (2026-07-18)

Gate: `./scripts/disk_persistence_gate.sh` (two boots, shared NVMe image). All rows PASS.

## What shipped

### 1. sexdisplay TEXT_MODEL_V2 (`servers/sexdisplay/src/main.rs`)
- `text_buf` 128 → 512 bytes (`TEXT_BUF_CAP`), `text_len` u8 → u16.
- Per-line index table (`line_starts[24]`, `line_count`, `wrap_cols`) recomputed
  on every 0xFB write. `'\n'` splits lines; wrap at width-derived cols
  (`(w-10)/6`, clamp 1..=80). A buffer with NO `'\n'` keeps legacy 20-col wrap,
  so every pre-V2 padded-row sender renders unchanged.
- OP_TEXT_DRAW arg2 bits 12-15 = byte_offset high nibble (offsets 256-511).
  Legacy senders leave them zero — fully backward compatible.
- Lowercase glyph fix: pre-V2 range guard rejected 0x61-0x7A before the
  uppercase fold ran, making typed lowercase invisible. Now folded correctly.

### 2. Disk-backed Linen (`servers/linen/src/main.rs`, `servers/silk-shell/src/main.rs`)
- `linen_publish_disk_objects()`: probes DiskFS fixed objects (path ids 0-2)
  via SLOT_STORAGE SELECT+STAT at boot, publishes each present object into the
  session table. `[linen.disk.publish.done] count=3` with NVMe attached;
  fails soft per path without it.
- Shell: `LINEN_REMOTE_REAL` gate — while snapshots return only seed fallback,
  re-fetch on each paint (linen publishes asynchronously after boot; a first
  snapshot can race it). Stops once real entries arrive.

### 3. Persistent Quil (`servers/quil/src/main.rs`)
- `quil_persist_save()/quil_persist_load()`: document persisted to DiskFS
  fixed object path_id 2 (`/disk/quil-object-v1`), header `"QP01"|len` at
  offset 0, content at offset 16. Rides existing DiskFS bridge opcodes.
- Palette SAVE writes RamFS + DiskFS; palette LOAD prefers the DiskFS copy
  (authoritative — the boot buffer proof re-seeds RamFS every boot and would
  otherwise mask the disk copy after reboot), RamFS as fallback.
- Proven: save 242 bytes boot 1 → reboot → `[quil.persist.load.ok] bytes=242`
  boot 2, restored doc rendered via text-model V2 (pixel row count ≈3000).

### 4. Spindle real `disk` command (`apps/spindle/src/main.rs`)
- First spindle command backed by a live storage roundtrip:
  `disk` lists which DiskFS fixed objects are present (SELECT+STAT per path).
  `[spindle.disk.command] found=3` with NVMe.
- `spindle_storage_sync()`: drains stale fire-and-forget replies (history
  persist) before issuing, retries enqueue (bounded), and polls the reply
  non-blocking with a yield budget.

## Known limitation discovered (NOT fixed — kernel domain, out of sprint scope)
**IPC request loss under storage contention.** A `pdx_call` to sexfiles can
return status=0 (enqueued) yet never be received by sexfiles when sexfiles is
mid-exchange with another client (observed repeatedly while linen's boot
publish held it: quil's and spindle's SELECT vanished; the caller then hung
forever in a blocking `pdx_listen_raw`). Mitigations shipped: bounded
non-blocking reply polls in quil (`pdx_storage_call_bounded`) and spindle
(300k-yield budget) so the PD survives and reports
`[quil.persist.reply.timeout]` / `[spindle.disk.reply.timeout]`; the gate
sequences user disk ops after `[linen.disk.publish.done]`. Root cause lives in
the kernel async-IPC queue and deserves its own lane.

## Gate-authoring trap (cost 3 debug cycles)
`wait_marker` matched STALE serial logs from the previous run: QEMU truncates
`-serial file:` only once IT starts, so a wait issued right after spawning
QEMU reads last run's log and passes instantly → keys fire during boot →
random-looking input losses. Fix: `: > "$log"` in `boot()` before starting
QEMU. Any future multi-boot gate must do the same.
