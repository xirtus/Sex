# SEXNET_NETDIAG_SOURCE3_PLAN_V1

Date: 2026-05-19
Phase: J (Tasks 48–51)
Status: PASS REVIEW ONLY (source3 netdiag proven without kernel/ABI/browser changes)

## Plan Questions

### 1. Where is current HAL NET_DIAG/source=2 truth emitted?

In `kernel/src/hal/pci.rs`:

| Static | Type | Purpose |
|--------|------|---------|
| `NET_DIAG_HTTP_STATUS` | `AtomicU32` | HTTP status code (200, 0, etc.) |
| `NET_DIAG_HTTP_BYTES` | `AtomicU32` | Total HTTP response bytes |
| `NET_DIAG_HTTP_SOURCE` | `AtomicU8` | 1=mock, 2=real (source=2) |
| `NET_DIAG_HTTP_BODY_LEN` | `AtomicU32` | Bounded body capture length |
| `NET_DIAG_HTTP_BODY` | `[u8; 64]` | Body capture buffer (capped at 64) |

These are set during the HAL diagnostic HTTP fetch path (TCP SYN→SYN-ACK→HTTP GET→response parse). The `get_net_diag()` and `get_net_diag_body_chunk()` functions expose these via syscall 52.

Marker: `[net.diag.static.set] status=... bytes=... ok=1 source=real`

### 2. Where is current sexnet source=3 HTTP GET proof emitted?

In `servers/sexnet/src/main.rs`:

| Marker | Content |
|--------|---------|
| `[sexnet.http.get.tx.proof.done]` | sent=1 tx_dd=1 ok=1 |
| `[sexnet.http.response.rx]` | bytes=N bounded=1 ok=1 |
| `[sexnet.http.status.proof.done]` | status=200 ok=1 |
| `[sexnet.http.body.proof.done]` | bytes=N ok=1 |
| `[sexnet.phaseI.readiness]` | established=1 payload_tx=1 source=3 ok=1 |

Body data is stored in `HTTP_BODY_PREFIX_BUF` / `HTTP_BODY_PREFIX_LEN` (bounded at 256 bytes).

### 3. Is there already a sexnet status/diagnostic output lane?

Yes, multiple lanes exist:
- **Static mock text**: `BODY_TEXT` = `"Hello SexOS HTTP OK"` (always available)
- **HAL source=2 body fetch**: Via `sys_net_diag(0..N)` syscall 52, only when `source==2`
- **PDX query route**: `SEXNET_GET_STATUS` with sub-selectors `SEXNET_HTTP_BODY_LEN` and `SEXNET_HTTP_BODY_CHUNK` — currently returns `BODY_BUF` (HAL source=2 body)
- **Source=3 HTTP body**: `HTTP_BODY_PREFIX_BUF` / `HTTP_BODY_PREFIX_LEN` (Phase I results, not yet exposed through PDX diagnostic route)

### 4. Does a syscall or PDX route already expose network status?

Yes. `SEXNET_GET_STATUS` (PDX opcode 0x200) with sub-selectors 0x209 (`SEXNET_HTTP_BODY_LEN`) and 0x20A (`SEXNET_HTTP_BODY_CHUNK`) already provides a status query route. This is the existing diagnostic lane — no new syscall needed.

### 5. Can Phase J be done by reporting/marking source=3 results without ABI changes?

**Yes.** Phase J adds bounded markers in `servers/sexnet/src/main.rs` only:
- `[sexnet.netdiag.source3.status]` — declares source=3 as primary
- `[sexnet.netdiag.source3.route]` — confirms existing route works
- `[sexnet.netdiag.source3.syscall.proof.done]` — status marker proof (no new syscall)
- `[sexnet.netdiag.source3.body]` — body proof from source=3 buffer
- `[sexnet.netdiag.source3.body.proof.done]` — body proof complete

No new syscalls. No ABI changes. No sex-pdx changes. No kernel changes.

### 6. What remains HAL/source=2 legacy?

- `NET_DIAG_HTTP_*` statics in `kernel/src/hal/pci.rs` remain untouched
- `get_net_diag()` and `get_net_diag_body_chunk()` continue to work
- Syscall 52 continues to report HAL diagnostic data
- `sexnet.dynamic_body.set` with `source=2` marker still fires on boot
- HAL DNS query build/TX/parse/cache remain operational

HAL source=2 is kept as **legacy/fallback**, NOT removed.

### 7. What must stay deferred to Phase K browser route?

- Browser SLOT_NET grant activation
- Collar network permission
- HTTP response → HTML subset feed
- Browser remote text render
- Browser tab remote status
- All `browser.*` markers remain `granted=0` / `fetched=0`

### 8. What STOP FIRST boundaries apply?

- Kernel edits: STOP FIRST
- sex-pdx/global ABI edits: STOP FIRST
- HAL NET_DIAG deletion: STOP FIRST
- Browser networking grant: STOP FIRST
- Browser code changes: STOP FIRST
- Collar permission changes: STOP FIRST
- TCP/HTTP wire logic redesign: STOP FIRST
- DNS migration to source=3: STOP FIRST
- General socket API: STOP FIRST

## Conclusion

**PASS REVIEW ONLY.** source3 netdiag can be proven via bounded source-code markers + existing PDX status route without kernel, ABI, browser, or HAL deletion changes.

### Source Ownership Classification

| Source | Classification | Status |
|--------|---------------|--------|
| source=3 | PRIMARY (sexnet HTTP GET) | Phase I IMPLEMENTED, Phase J markers added |
| source=2 | LEGACY/FALLBACK (HAL diagnostic) | Retained, not retired |
| source=1 | MOCK (built-in test) | Retained for offline proof |
| source=mixed | Not valid — source=3 is always primary when available | N/A |

## Required Doc Marker

```
[sexnet.phaseJ.plan.pass]
```

