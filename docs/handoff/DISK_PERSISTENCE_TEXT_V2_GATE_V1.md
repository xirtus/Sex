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

## IPC request loss — ROOT-CAUSED AND FIXED (follow-up sprint, same day)
The "vanished request" hangs were NOT primarily a kernel queue bug. Chain
(commit ee45af07, gate `scripts/ipc_defer_gate.sh`):

1. **Server reply-wait discard (the actual killer).** sexfiles'
   `diskfs_block_call` waited for sexdrive's reply with `pdx_listen_raw(0)`
   and DISCARDED every non-reply message as "stale startup message" — but
   those are live client requests arriving mid-NVMe-roundtrip. Same bug
   class independently present in linen's `pdx_storage_sync` (ate the
   shell's snapshot fetch → empty linen list, shell parked in fetch all
   session). Fix pattern: **defer stash + replay** — stash non-reply
   messages in a small ring, main serving loop drains it before listening
   (`[sexfiles.defer.stash/replay]`, `[linen.defer.stash/replay]`).
   **Any new server sync-wait loop MUST use this pattern.**

2. **Kernel ipc_ring was SPSC used as MPSC.** Every client enqueues into a
   server's single message_ring; two producers could claim the same slot
   and both return Ok (one message lost). Now CAS slot claim + per-slot
   publish seq; consumer treats claimed-but-unwritten slots as empty.

3. **DiskFS bridge selection now per-caller.** Interleaved clients (real
   once defers landed) clobbered the single global selected path_id.

4. **Client-side settle before sync probes.** Fire-and-forget calls issued
   earlier on the same event (spindle history persist on Enter) have
   replies still in flight; a single empty poll proves nothing. Spindle
   settles (consume until sustained quiet streak) before its disk probe.

## Follow-up features (same day)
- **LINEN_DISK_OPEN_V1** (commit 7ea22e0b): opening `disk-nquil-v1` from
  the Linen list sends `OP_QUIL_OPEN_DISK_DOC` (0x4A) to the real quil PD,
  which restores the doc from DiskFS. Gate rows: linen_disk_open_intent,
  quil_disk_doc_recv, quil_disk_doc_load (13/13 PASS).
- **SPINDLE_PAGING_V1** (commit 95a12c5c): PgUp/PgDn page the terminal
  scrollback (offset honored in content_render, clamped to ring); keys
  pass through the shared is_spindle_text_key filter — both dispatch
  paths, no dead-branch split.

## Gate-authoring trap (cost 3 debug cycles)
`wait_marker` matched STALE serial logs from the previous run: QEMU truncates
`-serial file:` only once IT starts, so a wait issued right after spawning
QEMU reads last run's log and passes instantly → keys fire during boot →
random-looking input losses. Fix: `: > "$log"` in `boot()` before starting
QEMU. Any future multi-boot gate must do the same.
