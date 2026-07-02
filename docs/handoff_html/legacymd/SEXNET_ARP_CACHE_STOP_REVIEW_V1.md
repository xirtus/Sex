# SEXNET_ARP_CACHE_STOP_REVIEW_V1

Date: 2026-05-19
Branch: master
Commit baseline: ceba1fc (net: gate phase A ARP host observe proof)

## Review Questions

### 1. Where does current ARP receive/validate/reply code live?

`servers/sexnet/src/main.rs`:
- Phase A one-shot ARP: lines ~1303–1543
  - `sexnet.arp.rx.poll.begin` → `sexnet.arp.proof.done`
- ARP cache + bounded repeated replies: lines ~1546–1766
  - `sexnet.arp.cache.poll.begin` → `sexnet.arp.cache.proof.done`
- L2 proof (reuses descriptors after ARP): lines ~1768–1902

### 2. Is there already an ARP cache?

**Yes.** Four static variables at lines 52–55:
```rust
static mut ARP_CACHE_MAC: [u8; 6] = [0u8; 6];
static mut ARP_CACHE_IP: [u8; 4] = [0u8; 4];
static mut ARP_CACHE_VALID: u8 = 0;
static mut ARP_CACHE_REPLY_COUNT: u32 = 0;
```

- Capacity: 1 entry (within spec range 1–4)
- Fields: MAC [u8; 6], IP [u8; 4], valid bit, reply counter
- No heap allocation
- Update on valid ARP request (oper=1) with matching TPA
- Deterministic replacement: single slot overwrite each learn
- No dynamic path strings, no routing semantics

### 3. Is current proof one-shot only or reusable?

**Reusable.** The cache loop targets `cache_replies < 2` (line 1563) and
runs as a bounded poll (max 100M iterations). Each iteration:
- Learns sender MAC/IP from incoming ARP request (lines 1600–1620)
- Builds ARP reply using cache in TX frame (lines 1640–1700)
- Posts reply to E1000 TX descriptor ring (lines 1701–1711)
- Polls until TX DD is set (lines 1720–1729)
- Increments reply counter and re-arms RX descriptor

Bounded loop. No unbounded search. Proof survives repeated boots.

### 4. What is the smallest fixed-cache design?

Already implemented: **1-entry cache** (single slot overwrite).
- Covers spec minimum (capacity 1–4).
- Slot 0 always used.
- No round-robin counter needed for 1-entry case.
- Enough to prove insert/reply/repeated-request behavior.

### 5. Can this be done without kernel/ABI/sex-pdx edits?

**Yes.** All cache logic is local to `servers/sexnet/src/main.rs`.
No kernel, sex-pdx, ABI, scheduler, PKRU, browser, display, or silk-shell
touches are required.

### 6. Can repeated ARP request proof be produced in TAP/QEMU?

**Yes** (with TAP backend + `e1000e` model).
- TAP lane: external host ARP stimulus → guest receives on sexnet NIC.
- Usernet lane: SLiRP ARP frames arrive on e1000e NIC path.
- Current implementation uses bounded poll (max 100M outer, 8×8 descriptors).
- Two reply target contract already proven in prior runs.

### 7. What are STOP FIRST boundaries?

All STOP FIRST rules are respected:
| Boundary | Status |
|----------|--------|
| No kernel edits | ✓ (none needed) |
| No sex-pdx/global ABI edits | ✓ |
| No scheduler/PKRU/time changes | ✓ |
| No browser/sexdisplay/silk-shell changes | ✓ |
| No IPv4 parser implementation | ✓ (Phase C) |
| No ICMP/UDP/DNS/TCP/HTTP | ✓ |
| No routing table | ✓ |
| No dynamic allocator | ✓ |
| No unbounded loops | ✓ |
| No weakening global fault scan | ✓ |
| No hiding runtime failures | ✓ |
| No fake PASS | ✓ |

## STOP Review Conclusion

**[sexnet.phaseB.stop_review.pass]**

The ARP cache runtime is already implemented, bounded, and gated.
Phase B requires only documentation (STOP review, gate+handoff, multi-request
proof/gate, rollup update) plus a thin multi-request gate addition.
No source code changes to `servers/sexnet/src/main.rs` are required.

## Existing Cache Markers Accepted

| Existing Marker | Maps To |
|----------------|---------|
| `sexnet.arp.cache.learn` | `sexnet.arp.cache.insert` |
| `sexnet.arp.cache.reply` | cache hit + tx |
| `sexnet.arp.cache.reply.dd` | tx descriptor completion |
| `sexnet.arp.cache.proof.done` | cache proof complete |

Hit/miss: implicit via learn+reply cycle (cache is 1-entry, overwrite on each learn,
reply built from cache contents).
Reject: invalid ARP packets do not trigger cache learn (validity gate at line 1592).
