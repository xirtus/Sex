# SEXNET_PASSIVE_SPAWN_V1

**Status:** PASS IMPLEMENTED
**Date:** 2026-05-16
**Proof:** 121 gates PASS, 0 FAIL, 0 faults

---

## Safety Verdict

PASS SAFE — all STOP FIRST conditions clear before implementation.

| Check | Status |
|-------|--------|
| sex-pdx edit | ❌ None |
| Global ABI edit | ❌ None |
| Scheduler/memory change | ❌ None |
| Blocking hardware wait | ❌ None (mock PDX loop) |
| NIC/QEMU net device | ❌ None |
| Browser SLOT_NET grant | ❌ None |
| Browser network=1/fetched=1 | ❌ None |
| Broad boot refactor | ❌ None |

---

## Boot/Staging Changes

| File | Change |
|------|--------|
| `sexos_build_spec.toml` | Added `servers/sexnet/Cargo.toml` to `[allowed].crates`; added `[[stage]] id=build_sexnet` |
| `limine.cfg` | Added `MODULE_PATH=boot:///servers/sexnet` |
| `Cargo.toml` | Added `servers/sexnet` to workspace members |
| `kernel/src/init.rs` | Added `sexnet_id`, `"sexnet"` to `module_paths` (index 12, domain_id 13), `else if domain_id == 13` branch, `[kernel.sexnet.passive]` marker |

---

## sexnet Liveness Table

| Marker | Value |
|--------|-------|
| `[sexnet.boot]` | `ok=1 reason=passive_spawn` |
| `[sexnet.passive.ready]` | `network=0 dns=0 tcp=0 http=0 tls=0 ok=1 reason=mock_status_only_no_nic` |
| `[sexnet.passive.spawn.done]` | `ok=1 spawned=1 browser_network=0` |
| `[kernel.sexnet.passive]` | `spawned=1 id=13 slot_net_grant=0 browser_network=0` |

---

## Browser Network Truth Table

| Field | Value | Source |
|-------|-------|--------|
| `spawned` | 1 | sexnet running at domain_id 13 |
| `slot_net_grant` | 0 | No SLOT_NET granted to Browser |
| `network` | 0 | No NIC, no real TCP/IP |
| `fetched` | 0 | No HTTP requests |
| `dns` | 0 | No DNS |
| `http` | 0 | No HTTP client |
| `tls` | 0 | No TLS |
| `ok` | 1 | Passive spawn, no capability increase |

Markers emitted from silk-shell:
- `[sexnet.stub.status] spawned=1 slot_net=0 nic=0 tcp=0 dns=0 http=0 ok=1`
- `[browser.net.status] sexnet=1 slot_net=0 network=0 fetched=0 ok=1`
- `[browser.sexnet.truth] spawned=1 slot_net_grant=0 network=0 fetched=0 dns=0 http=0 tls=0 ok=1`

---

## Proof Result

```
PASS gates: 121
FAIL gates: 0
SKIP gates: 0
FINAL: PASS (121 gates proved, 0 skipped, 0 faults)
```

Previous baseline: 120 gates. New gate added: `sexnet_passive`.

---

## Fault Count

0 fault markers in boot log.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/sexnet/src/main.rs` | Fixed stale API (SexnetApEntry, SEXNET_* constants now local; PdxMessage fields updated; removed LockedHeap/alloc conflict; added boot markers) |
| `kernel/src/init.rs` | Added sexnet_id, module_paths entry, domain_id 13 branch, passive marker |
| `limine.cfg` | Added sexnet MODULE_PATH |
| `sexos_build_spec.toml` | Added sexnet to allowed crates + build stage |
| `Cargo.toml` | Added servers/sexnet to workspace |
| `servers/silk-shell/src/main.rs` | Updated spawned=0→1, added browser.sexnet.truth marker |
| `scripts/daily_driver_master_gate.sh` | Added gate_sexnet_passive variable, check, and ALL_GATES entry |

---

## Next Recommendation

**SEXNET_STATUS_ROUTE_PROOF_V1** — Add a proof that a PDX status call to sexnet returns valid mock data (GET_STATUS → 0 = disconnected). No NIC, no network packets. Verifies the PDX listen loop is live and responsive.

Alternative: **BROWSER_NETWORK_GRANT_STUB_V1** — Add a collar-gated stub that attests no SLOT_NET capability can be granted to Browser without explicit collar approval. Proof-only, no actual grant.
