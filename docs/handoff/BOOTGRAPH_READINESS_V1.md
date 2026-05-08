# BOOTGRAPH_READINESS_V1

- date: 2026-05-08
- status: DESIGN_DOC — no code changes
- scope: docs only

---

## Purpose

BootGraph tracks which Protection Domains (PDs) are ready before dependents start work.
No POSIX. No systemd. No runit. Pure PDX + marker-based coordination.

Goal: eliminate the class of bugs where a sender fires before the receiver has set up its
PDX receive loop, cap table, or display surface. Every PD declares what it needs; the graph
enforces order via runtime markers (V1), PDX handshake (V2), or mesh-visible state (V3).

---

## Current Boot Dependency Graph

Kernel spawns PDs in array order from `init.rs`:

```
sexdisplay → (no upstream PD dep — owns framebuffer directly)
sexdrive   → (no upstream PD dep — owns block device)
silk-shell → display=sexdisplay, bar=silkbar, bell=sexbell, linen=linen
sexinput   → shell=silk-shell
sexusb     → input=sexinput
silkbar    → display=sexdisplay, bell=sexbell
linen      → display=sexdisplay, storage=sexfiles
sexstore   → (standalone)
quil       → storage=sexfiles (optional)
sexbell    → (standalone)
sexfiles   → (standalone)
spindle    → (standalone, user-facing app)
```

Capability grants happen after all spawns complete (Phase 25 in init.rs). A PD that calls
`pdx_call` before its cap is granted will see a null slot and silently drop the message or fault.

---

## Readiness Markers Per PD

Each PD emits a serial marker when it has completed initialization and is safe to receive
messages on all its registered slots. These are the V1 canonical markers:

| PD         | Readiness Marker                          | What it means                                 |
|------------|-------------------------------------------|-----------------------------------------------|
| sexdisplay | `[sexdisplay.ready]`                      | Framebuffer mapped, receive loop running      |
| sexdrive   | `[sexdrive.ready]`                        | Block device open, PDX loop running           |
| silk-shell | `[silkshell.ready]`                       | Focus state init, PDX loop running            |
| sexinput   | `[sexinput.ready]`                        | HID table init, PDX loop running              |
| sexusb     | `[sexusb.ready]`                          | xHCI ring init, interrupt-IN loop running     |
| silkbar    | `[silkbar.ready]`                         | Clock arm, contract validated, loop running   |
| linen      | `[linen.ready]`                           | Session state init, loop running              |
| sexstore   | `[sexstore.ready]`                        | KV store open, loop running                   |
| quil       | `[quil.ready]`                            | Disk FS mount complete, loop running          |
| sexbell    | `[sexbell.ready]`                         | Bell ring buffer init, loop running           |
| sexfiles   | `[sexfiles.ready]`                        | RamFS/NVMe mount, loop running                |
| spindle    | `[spindle.ready]`                         | TUI frame init, PDX loop running              |

Marker format: printed via `serial_println!` in `_start` before the first blocking `pdx_recv`.

---

## Dependency Matrix

Rows = sender. Columns = receiver. `X` = sender must wait for receiver marker before first call.

|            | sexdisplay | sexdrive | silk-shell | sexinput | sexusb | silkbar | linen | sexstore | quil | sexbell | sexfiles | spindle |
|------------|:----------:|:--------:|:----------:|:--------:|:------:|:-------:|:-----:|:--------:|:----:|:-------:|:--------:|:-------:|
| silk-shell |     X      |          |            |          |        |    X    |   X   |          |      |    X    |          |         |
| sexinput   |            |          |     X      |          |        |         |       |          |      |         |          |         |
| sexusb     |            |          |            |    X     |        |         |       |          |      |         |          |         |
| silkbar    |     X      |          |            |          |        |         |       |          |      |    X    |          |         |
| linen      |     X      |          |            |          |        |         |       |          |      |         |    X     |         |
| quil       |            |          |            |          |        |         |       |          |      |         |    X     |         |
| spindle    |            |          |     X      |          |        |         |       |          |   X  |         |          |         |

---

## V1 — Marker-Only (current baseline)

Implementation: serial log scan only. No runtime enforcement.

Mechanism:
1. Each PD emits `[pd.ready]` marker at `_start` before `pdx_recv`.
2. Gate scripts (`master_runtime_gate.sh`) scan serial log for required markers.
3. If a marker is absent → gate = RED → do not ship.

Rules for V1:
- Markers must appear before the first blocking call.
- No sentinel message required — log presence is sufficient proof.
- Marker drift (wrong string) treated same as absent marker.
- Canonical alias table (see below) is source of truth.

V1 limitations:
- Log-only: a PD can emit the marker then deadlock before its receive loop is live.
- No ordering enforcement between PDs at runtime.
- Suitable only for CI gate and triage, not production readiness.

---

## V2 — PDX Handshake (next phase)

Mechanism:
1. Dependent PD sends `OP_PING` to upstream on startup.
2. Upstream responds `OP_PONG` only after its receive loop is live.
3. Dependent defers all upstream calls until pong received.
4. Timeout (e.g. 5000 ticks) → dependent logs `[boot.wait.timeout pd=X]` and continues degraded.

Rules for V2:
- `OP_PING` / `OP_PONG` reserved opcodes, not to be reused.
- Handshake must complete before dependent emits its own `[pd.ready]` marker.
- No circular ping dependencies allowed.
- Degraded mode must be defined per PD (e.g. silkbar skips clock if sexdisplay times out).

Required opcode additions (STOP FIRST before adding):
- `OP_PING = 0xFE`
- `OP_PONG = 0xFF`
- Must not collide with existing surface opcodes (see `claude-references/SILKBAR_ABI.md`).

---

## V3 — Mesh-Visible Graph (future phase)

Mechanism:
1. Each PD writes readiness state into a shared mesh (capability-gated read-only segment).
2. Kernel or sexdisplay renders boot progress overlay on framebuffer.
3. Dependents poll mesh via PDX read-cap instead of polling serial log.
4. Boot graph visible to user as animated progress indicator.

Rules for V3:
- Mesh segment is read-only to all PDs except the owning PD.
- No raw pointer sharing — all access via PDX cap.
- Mesh layout versioned: bump version field before any layout change.
- PKU key assigned to mesh segment at init; kernel opens key at entry, closes at return.

---

## Rules for Deferring Startup Work

1. **Do not call a slot until the receiver's readiness marker has been observed.**
   In V1: rely on spawn order + serial log confirmation.
   In V2: defer until `OP_PONG` received.

2. **Do not emit your own `[pd.ready]` until all upstream handshakes complete.**
   Downstream PDs treat your marker as authorization to call you.

3. **Large initialization work (RamFS scan, NVMe mount, font load) must complete before ready marker.**
   The marker is a contract: once emitted, the PD is live and responsive.

4. **If upstream is optional (e.g. sexbell absent), emit ready marker anyway.**
   Log `[boot.optional.absent pd=sexbell]` and continue without that capability.

5. **Never spin-wait for a readiness marker inside a PD.**
   Use `pdx_yield` or a timed retry loop with bounded iteration count.
   Unbounded spin is a starvation risk for the cooperative scheduler.

---

## Runtime Gate Requirements

For a build to pass BOOTGRAPH gate (extends SPAWN_GATE):

| Check | Requirement |
|-------|-------------|
| All 12 PD ready markers present | PASS |
| No PD emits ready marker before its upstream markers appear in log | PASS |
| No marker appears after a `fault.kill` for that PD | PASS |
| Marker strings match canonical alias table exactly | PASS |

Canonical marker alias table (single source of truth):

```
sexdisplay  → [sexdisplay.ready]
sexdrive    → [sexdrive.ready]
silk-shell  → [silkshell.ready]
sexinput    → [sexinput.ready]
sexusb      → [sexusb.ready]
silkbar     → [silkbar.ready]
linen       → [linen.ready]
sexstore    → [sexstore.ready]
quil        → [quil.ready]
sexbell     → [sexbell.ready]
sexfiles    → [sexfiles.ready]
spindle     → [spindle.ready]
```

Any deviation (e.g. `[silkshell.init.done]` instead of `[silkshell.ready]`) is MARKER_DRIFT —
see RECOVERY_BRAIN_TRAINING_LEDGER.md failure class 2.
