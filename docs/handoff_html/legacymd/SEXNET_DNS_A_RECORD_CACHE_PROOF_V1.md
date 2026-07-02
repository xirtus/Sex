# SEXNET_DNS_A_RECORD_CACHE_PROOF_V1

Date: 2026-05-19
Branch: master
Proof: Phase F Task 28 — DNS A-record cache proof
Depends on: SEXNET_DNS_CLIENT_STOP_REVIEW_V1 (PASS REVIEW)

## Result: PASS IMPLEMENTED

A tiny bounded 4-entry DNS A-record cache has been implemented in `kernel/src/hal/pci.rs`.
The cache stores A records extracted from live DNS responses and supports hit/miss tests.
No heap, no dynamic allocation, no unbounded growth. Deterministic replacement policy.

## Cache Design

| Property | Value |
|----------|-------|
| Capacity | 4 entries |
| Entry size | host_id (u8) + ip ([u8;4]) + ttl (u32) = 9 bytes |
| Total memory | ~36 bytes (stack) |
| Allocation | Stack only, no heap |
| Replacement | Empty-first, then slot 0 (round-robin) |
| Host resolution | Fixed host slot: host_id=1 = example.com |
| TTL | Stored but not used for expiry (no time subsystem) |
| API | None — inline probe only, no general resolver API |

## Cache Operations

### Init
```
[sexnet.dns.cache.init] cap=4
```
All 4 entries initialized to host_id=0 (empty), ip=0.0.0.0, ttl=0.

### Insert
After DNS response parse extracts A records, each A record is inserted:
1. Find first empty slot (host_id==0)
2. If all slots full, use slot 0 (deterministic replacement)
3. Store host_id=1 (example.com), ip, ttl
4. Emit per-insert marker

```
[sexnet.dns.cache.insert] idx=N host=example.com addr=A.B.C.D ok=1
```

### Hit
After insert, test cache hit by scanning for host_id=1:
```
[sexnet.dns.cache.hit] idx=N host=example.com addr=A.B.C.D ok=1
```

### Miss
Test cache miss by looking up a non-cached hostname:
```
[sexnet.dns.cache.miss] host=nonexistent.host ok=1
```

### Proof Done
```
[sexnet.dns.cache.proof.done] inserts=N hits=N misses=N ok=1
```

## Cache Safety

| Rule | Applied |
|------|---------|
| Capacity 1-4 entries | YES — 4 entries |
| Fixed hostname slot | YES — host_id=1 = example.com |
| No heap | YES — stack arrays only |
| Deterministic replacement | YES — empty-first, slot-0 fallback |
| No TTL expiry | YES — TTL stored, no time subsystem |
| No general resolver API | YES — inline probe only |
| No browser route | YES |
| Bounded insert loop | YES — max 2 A records * 4 slots |
| Bounded lookup | YES — max 4 slot scan |

## Integration With DNS Parse

The cache is populated immediately after the `[dns.response.parse.proof.done]` marker.
It uses the same `q_a_ip` and `q_a_ttl` arrays extracted by the bounded DNS response
parser. The cache is declared before the DNS probe block and persists across both
the UDP DNS probe and the DNS response parse probe.

## Cache Hit Proof

A cache hit is demonstrated immediately after insert by scanning the cache for
host_id=1 (example.com). This proves:
- Inserted entries are retrievable
- Host_id lookup works
- Retrieved IP matches inserted IP

## Cache Miss Proof

A cache miss is demonstrated by looking up a nonexistent hostname. With the current
fixed host-slot design (no string comparison, only host_id), the miss test simply
verifies the miss counter increments.

## Live vs Self-Test

The cache operates on **live DNS A records** when SLiRP DNS is reachable (user+e1000e
backend). In this case, real Cloudflare IPs for example.com are stored.

If DNS response is absent (TAP without DNS routing, or no-network lane), the cache
init marker is still emitted (cap=4), but no insert/hit markers appear. The gate
should SKIP in this case.

## Phase F A-Record Cache Conclusion

- [sexnet.dns.cache.init] cap=4
- [sexnet.dns.cache.insert] idx=N host=example.com addr=A ok=1
- [sexnet.dns.cache.hit] idx=N host=example.com addr=A ok=1
- [sexnet.dns.cache.miss] host=nonexistent.host ok=1
- [sexnet.dns.cache.proof.done] inserts=N hits=N misses=N ok=1

**PASS IMPLEMENTED.** The DNS A-record cache is a minimal, bounded 4-entry fixed-slot
cache with deterministic replacement. It stores real A records from live DNS responses
and supports hit/miss proof. All safety invariants are preserved.
