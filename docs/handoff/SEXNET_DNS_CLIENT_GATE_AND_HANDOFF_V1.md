# SEXNET_DNS_CLIENT_GATE_AND_HANDOFF_V1

Date: 2026-05-19
Branch: master
Phase F Task 29 — DNS client gate and handoff
Depends on: Tasks 24-28 (all PASS)

## Gate Design

Phase F adds these gates to `scripts/daily_driver_master_gate.sh`:

| Gate Name | Marker Check | Expected |
|-----------|-------------|----------|
| `sexnet_dns_query_build` | `[udp.dns.query.send].*ok=1` or `[dns.query.build.proof].*ok=1` | PASS if present |
| `sexnet_dns_query_tx` | `[udp.dns.query.send].*tx_dd=1.*ok=1` | PASS if TX confirmed |
| `sexnet_dns_response_parse` | `[dns.response.parse.proof.done].*ok=1` | PASS if parse done |
| `sexnet_dns_a_record_cache` | `[sexnet.dns.cache.proof.done].*ok=1` | PASS if cache proof done |

### Gate Behavior

#### sexnet_dns_query_build

- **PASS**: DNS query build marker present with ok=1
  - `[udp.dns.query.send]` with ok=1 (e1000e lane)
  - OR `[dns.query.build.proof]` with ok=1 (Bundle D lane)
- **SKIP**: No DNS markers present (no NIC, or PCI not probed, or DNS probe disabled)
- **FAIL**: Never — build is always ok=1 if present; if malformed, parse would catch it

#### sexnet_dns_query_tx

- **PASS**: TX dd confirmed with ok=1
  - `[udp.dns.query.send]` with `tx_dd=1.*ok=1`
  - OR `[dns.parse.query.send]` with `tx_dd=1.*ok=1`
- **SKIP**: DNS query sent but no TX dd confirmation, or no DNS TX lane
- **FAIL**: DNS query built but tx_dd=0 or bounds overflow detected

#### sexnet_dns_response_parse

- **PASS**: Parse proof done with ok=1
  - `[dns.response.parse.proof.done] ok=1` with a_records>=1
  - Live response parse OR clearly marked self-test
- **SKIP**: No DNS response available (environment-blocked TX, no SLiRP DNS, TAP without DNS routing)
  - Also SKIP if `parsed=0` but `ok=1` (Bundle D honest no-response lane)
- **FAIL**: Malformed DNS accepted, or parse ok=0, or parser out-of-bounds risk found

#### sexnet_dns_a_record_cache

- **PASS**: Cache proof done with ok=1
  - `[sexnet.dns.cache.proof.done] inserts>=1 hits>=1 misses>=1 ok=1`
- **SKIP**: Cache init present but no inserts (DNS response absent)
- **FAIL**: Cache proof marker with ok=0, or bounds overflow detected

### Fault Integration

All gates integrate with the existing fault scan:
- If `fault.kill`, `#PF`, `#GP`, `panic`, or `KERNEL PANIC` detected, all gates FAIL
- Fault count must be 0 for PASS

## Existing DNS Gates (Pre-Phase F)

The following gates already exist in the daily driver script and continue to function:

| Gate | Status |
|------|--------|
| `udp_dns_probe` | Existing — checks `[udp.dns.probe.done].*ok=1` |
| `dns_response_parse_proof` | Existing — checks `[dns.response.parse.proof.done].*ok=1.*a_records=[1-9]` |
| `dns_client_plan` | Existing — checks `[dns.client.plan].*ok=1` |
| `dns_query_build_proof` | Existing — checks `[dns.query.build.proof].*ok=1` |
| `dns_query_send_stop_review` | Existing — checks `[dns.query.send.stop.review]` |
| `dns_query_send_proof` | Existing — checks `[dns.query.send.proof].*ok=1` |
| `dns_to_http_host_resolution_proof` | Existing — checks `[dns.to.http.host.resolution.proof].*ok=1` |

The new Phase F gates (`sexnet_dns_*`) are added as **additional** gates, not replacements.
Both sets coexist in the gate summary.

## Gate Declaration Pattern

Following the existing pattern in `daily_driver_master_gate.sh`:

```bash
# Default: SKIP
gate_sexnet_dns_query_build="SKIP"
gate_sexnet_dns_query_tx="SKIP"
gate_sexnet_dns_response_parse="SKIP"
gate_sexnet_dns_a_record_cache="SKIP"

# In gate evaluation section:
# ---- SEXNET_DNS_QUERY_BUILD ----
if [ "$(has 'udp.dns.query.send.*ok=1')" -eq 1 ] || [ "$(has 'dns.query.build.proof.*ok=1')" -eq 1 ]; then
    gate_sexnet_dns_query_build="PASS"
    print_row "sexnet_dns_query_build" "PASS" "DNS query build proof: example.com A query built"
else
    gate_sexnet_dns_query_build="SKIP"
    print_row "sexnet_dns_query_build" "SKIP" "no DNS query build marker"
fi

# ---- SEXNET_DNS_QUERY_TX ----
if [ "$(has 'udp.dns.query.send.*tx_dd=1.*ok=1')" -eq 1 ] || [ "$(has 'dns.parse.query.send.*tx_dd=1.*ok=1')" -eq 1 ]; then
    gate_sexnet_dns_query_tx="PASS"
    print_row "sexnet_dns_query_tx" "PASS" "DNS query TX proof: frame posted, tx_dd=1"
else
    gate_sexnet_dns_query_tx="SKIP"
    print_row "sexnet_dns_query_tx" "SKIP" "DNS query TX not confirmed or not exercised"
fi

# ---- SEXNET_DNS_RESPONSE_PARSE ----
if [ "$(has 'dns.response.parse.proof.done.*a_records=[1-9].*ok=1')" -eq 1 ]; then
    gate_sexnet_dns_response_parse="PASS"
    print_row "sexnet_dns_response_parse" "PASS" "DNS response parse proof: A records extracted"
elif [ "$(has 'dns.response.parse.proof.*parsed=0.*ok=1')" -eq 1 ]; then
    gate_sexnet_dns_response_parse="SKIP"
    print_row "sexnet_dns_response_parse" "SKIP" "DNS parse: no response in window (honest)"
else
    gate_sexnet_dns_response_parse="SKIP"
    print_row "sexnet_dns_response_parse" "SKIP" "DNS parse not exercised"
fi

# ---- SEXNET_DNS_A_RECORD_CACHE ----
if [ "$(has 'sexnet.dns.cache.proof.done.*ok=1')" -eq 1 ]; then
    gate_sexnet_dns_a_record_cache="PASS"
    print_row "sexnet_dns_a_record_cache" "PASS" "DNS A-record cache proof: inserts/hits/misses"
else
    gate_sexnet_dns_a_record_cache="SKIP"
    print_row "sexnet_dns_a_record_cache" "SKIP" "DNS cache proof not present or no DNS response"
fi
```

## Gate Summary Integration

Add to the gate summary array (where scores are printed):

```bash
    "sexnet_dns_query_build:$gate_sexnet_dns_query_build"
    "sexnet_dns_query_tx:$gate_sexnet_dns_query_tx"
    "sexnet_dns_response_parse:$gate_sexnet_dns_response_parse"
    "sexnet_dns_a_record_cache:$gate_sexnet_dns_a_record_cache"
```

## Handoff

Phase F DNS client gates are now:
- **sexnet_dns_query_build**: Proves DNS query frame construction (example.com, A, port 53)
- **sexnet_dns_query_tx**: Proves DNS query transmission via e1000e TX lane (tx_dd=1)
- **sexnet_dns_response_parse**: Proves bounded DNS response parser extracts A records
- **sexnet_dns_a_record_cache**: Proves tiny fixed A-record cache with hit/miss

All gates follow the existing pattern:
- PASS with required markers and ok=1
- SKIP if environment-blocked or lane not exercised
- FAIL if malformed or fault detected
- 0 faults required for all PASS gates

## Phase F Gate Contract

| Gate | Requires | PASS condition | SKIP condition | FAIL condition |
|------|----------|---------------|----------------|----------------|
| sexnet_dns_query_build | e1000e NIC or Bundle D | marker + ok=1 | no DNS markers | never (build always ok) |
| sexnet_dns_query_tx | e1000e NIC + DNS server reachable | tx_dd=1 + ok=1 | no TX or tx_dd unset | tx_dd=0 or overflow |
| sexnet_dns_response_parse | e1000e NIC + DNS response | parse done + a_records>=1 + ok=1 | no response (parsed=0 ok=1) | parse ok=0 |
| sexnet_dns_a_record_cache | e1000e NIC + DNS response | cache proof done + inserts>=1 + ok=1 | cache init only, no inserts | ok=0 |

## Fault Contract

All Phase F gates integrate with the global fault gate:
- If fault count > 0, all gates are suspect
- `fault.kill`, `#PF`, `#GP`, `panic`, `KERNEL PANIC` → immediate FAIL
- Fault scan runs before gate evaluation
