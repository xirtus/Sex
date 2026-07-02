# SEXNET_BROWSER_ROUTE_AUDIT_V1

**Status:** PASS REVIEW ONLY — No network route from Browser to sexnet.
**Date:** 2026-05-16
**Gates:** 119/119 baseline.

---

## Network Reality Table

| Component | Exists? | Notes |
|-----------|---------|-------|
| sexnet server | ✅ `servers/sexnet/` | WiFi state, VPN state, mock AP scan, uses alloc |
| silknet crate | ✅ `crates/silknet/` | Opcodes: GET_STATUS, SCAN_WIFI, CONNECT, DISCONNECT, VPN_UP/DOWN, GET_IP |
| sexnet spawned at boot | ❌ | Not in `kernel/src/init.rs` module_paths |
| Browser→sexnet slot | ❌ | No SLOT_NET grant |
| Kernel net syscalls | ✅ `kernel/src/syscalls/net.rs` | sys_socket, sys_send, route_net_call — unused by any PD |
| NIC driver | ❌ | Mock AP scan only, no real NIC |
| TCP/IP stack | ❌ | No implementation |
| DNS resolver | ❌ | No implementation |
| HTTP client | ❌ | No implementation |
| TLS | ❌ | No implementation |
| QEMU net device | ❌ | Not configured in QEMU command |
| Collar network grant | ❌ | Collar not spawned |

---

## Route/Capability Table

| Path | Status |
|------|--------|
| Browser → sexnet | ❌ No SLOT_NET grant |
| Browser → kernel net syscalls | ❌ No syscall dispatch from PDs |
| sexnet → NIC | ❌ Mock only |
| Collar → network grant | ❌ Collar not spawned |

---

## Blockers

1. **sexnet not spawned** — add to `kernel/src/init.rs` module_paths
2. **No SLOT_NET grant** — grant Browser access to sexnet
3. **No NIC driver** — sexnet uses mock AP scan
4. **No TCP/IP** — needs full stack or minimal TCP
5. **No DNS** — static IP or hosts-based
6. **No HTTP client** — needs GET parser + response buffer
7. **No QEMU net** — needs `-netdev user,id=n0 -device e1000,netdev=n0`
8. **Collar not spawned** — network grant authority doesn't exist

---

## Recommended Next: **A — SEXNET_BROWSER_CAPABILITY_STUB_V1**

Marker-only. Spawn sexnet, add SLOT_NET stub to Browser. Keep network=0, no packets. Prove the route exists structurally before implementing TCP/HTTP.

---

## STOP FIRST Boundaries for Future Fetch

| Boundary | Status |
|----------|--------|
| Kernel/ABI edits | ❌ kernel net syscalls already exist |
| POSIX sockets | ❌ PDX-native only |
| Heap/thread | ⚠️ sexnet uses alloc — acceptable for server |
| Unbounded buffers | ❌ bounded by design |
| Fetch without Collar grant | ❌ must gate behind Collar |
| Browser direct NIC | ❌ must route through sexnet |

---

## Next Prompt: SEXNET_BROWSER_CAPABILITY_STUB_V1

## Commit
```bash
git add docs/handoff/SEXNET_BROWSER_ROUTE_AUDIT_V1.md
git commit -m "docs(net): sexnet browser route audit V1"
```
