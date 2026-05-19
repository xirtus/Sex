# SEXNET_TCP_STATE_MACHINE_STOP_REVIEW_V1

Date: 2026-05-19
Phase: G (TCP handshake)
Review: STOP review before TCP implementation

## STOP Review Questions

### 1. Where does current IPv4 protocol=6/TCP path live, if any?

**Answer:** No TCP code exists in `servers/sexnet/src/main.rs`. The IPv4 RX path handles:
- proto=1 (ICMP echo reply)
- proto=17 (UDP echo reply)
- proto=6 is NOT handled.

The kernel HAL diagnostic lane (`kernel/src/hal/pci.rs`) has TCP markers at source=2 with `[tcp.syn.build.proof]`, `[tcp.syn.send.stop.review]`, and `[tcp.handshake.proof]` markers. These are plan-only / deferred markers, not a working TCP implementation.

### 2. Are current TCP proofs in sexnet-owned path or HAL diagnostic/source=2?

**Answer:** All existing TCP markers are in the HAL diagnostic lane (source=2) in `kernel/src/hal/pci.rs`:
- `[tcp.minimal.state.machine.plan]` — plan only
- `[tcp.syn.build.proof]` — bounded SYN shape only, no TX
- `[tcp.syn.send.stop.review]` — explicitly stopped, no TX post
- `[tcp.handshake.proof]` — observed=0, no SYN-ACK peer capture

Sexnet server has NO TCP code at all.

### 3. Is SYN build already implemented?

**Answer:** Not in sexnet. HAL diagnostic has a plan-level `[tcp.syn.build.proof]` marker (shape only, no actual packet construction). No TCP header, pseudo-header checksum, or SYN flag exists in any source=3 path.

### 4. Is TCP checksum already implemented?

**Answer:** No TCP checksum code exists anywhere. UDP pseudo-header checksum code exists in the IPv4 RX path (proto=17) and can be adapted for TCP (proto=6, same pseudo-header structure, different protocol number).

### 5. Is SYN TX already proven with TX DD?

**Answer:** No. HAL diagnostic has `stop=1` markers explicitly preventing SYN TX. No SYN TX descriptor write, TDT post, or DD poll for TCP exists.

### 6. Is SYN-ACK RX or RST RX already observed?

**Answer:** No. HAL diagnostic explicitly records `observed=0 reason=no_synack_peer_capture_in_phase`. No TCP header parsing exists in the RX path.

### 7. Is final ACK TX already implemented/proven?

**Answer:** No. No ACK build or TX exists anywhere.

### 8. Can Phase G complete without TCP payload/HTTP/browser?

**Answer:** Yes. Phase G scope is explicitly limited to:
- TCP SYN build and TX
- TCP SYN-ACK RX and validation (or honest RST/timeout)
- Final ACK TX (if SYN-ACK observed)
- Minimal state machine (CLOSED → SYN_SENT → ESTABLISHED)
- No TCP payload, no PSH/ACK data, no HTTP, no browser

### 9. Can this be done without kernel/ABI/sex-pdx edits?

**Answer:** Yes. TCP handshake can be added entirely within `servers/sexnet/src/main.rs`:
- New code uses existing TX descriptor infrastructure (desc 5, TDT=6 for SYN; desc 6, TDT=7 for ACK)
- New proto=6 handler uses existing IPv4 RX path
- No new syscalls or kernel changes needed
- NIC ownership (source=3) already proven in Phases A through E

### 10. What STOP FIRST boundaries apply?

**Answer:**
- **No kernel edits** — all TCP code in sexnet server
- **No sex-pdx/global ABI edits** — existing PDX interface sufficient
- **No scheduler/PKRU/time changes**
- **No browser/sexdisplay/shell changes**
- **No HTTP** — Phase G is TCP only
- **No TCP payload** — SYN/ACK have zero payload
- **No general socket API**
- **No multi-connection table** — one connection only
- **No source=3 migration** — new code is source=3 in sexnet; HAL diagnostic source=2 markers preserved as-is
- **HAL NET_DIAG retirement** — deferred to future phase
- **All polls bounded** — max 50M iterations per DD poll
- **Bounded SYN retries** — max 3 SYN sends if no response

## TCP Minimal State Contract

| State | Description |
|-------|-------------|
| CLOSED | Initial state, no connection |
| SYN_SENT | SYN transmitted, awaiting SYN-ACK |
| ESTABLISHED | SYN-ACK received, ACK sent |
| FAILED_RST | RST received, connection refused |
| FAILED_TIMEOUT | No SYN-ACK/RST within bounded retries |

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Local port | 7777 | Deterministic, unused by other services |
| Remote port | 80 | Well-known HTTP port; no HTTP sent, just handshake |
| Remote IP | 10.0.2.2 | Gateway (usernet) or host (TAP) |
| Local sequence | 42 | Deterministic, simple |
| Remote sequence | From SYN-ACK | Parsed from received segment |
| SYN flags | SYN=1, ACK=0 | Standard TCP SYN |
| SYN-ACK flags | SYN=1, ACK=1 | Standard TCP SYN-ACK |
| ACK flags | ACK=1 | Standard TCP ACK |
| data_offset | 5 | 20-byte header, no options |
| window | 65535 | Maximum window |
| TCP checksum | Computed over pseudo-header + TCP header | Standard TCP checksum |

## STOP Review Conclusion

**[sexnet.phaseG.stop_review.pass]**

Implementation is safe to proceed. TCP handshake can be added entirely within sexnet server using existing infrastructure (NIC TX descriptors, IPv4 RX path, TX frame buffer). No forbidden edits needed. Source ownership will be sexnet source=3 for new TCP code, coexisting with HAL diagnostic source=2 markers that remain as-is. Phase G scope is minimal: one connection, SYN → SYN-ACK → ACK, no payload, no HTTP.

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| No SYN-ACK in usernet | Honest SKIP/PASS REVIEW ONLY if environment doesn't route TCP |
| No SYN-ACK in TAP without listener | Honest SKIP; gate can PASS REVIEW ONLY |
| Checksum error in TX | Validated via same pseudo-header code as UDP (proven) |
| Descriptor collision | Use desc 5/6, distinct from desc 0-4 |
| Unbounded retry | Hard-coded max 3 SYN sends |
| RST from remote | Handle honestly, FAILED_RST state |

## File Plan

| File | Change |
|------|--------|
| `servers/sexnet/src/main.rs` | ADD: TCP state machine, SYN build/TX, SYN-ACK RX, ACK TX |
| `docs/handoff/SEXNET_TCP_SYN_BUILD_PROOF_V1.md` | CREATE |
| `docs/handoff/SEXNET_TCP_SYN_TX_PROOF_V1.md` | CREATE |
| `docs/handoff/SEXNET_TCP_SYNACK_RX_PROOF_V1.md` | CREATE |
| `docs/handoff/SEXNET_TCP_ACK_TX_PROOF_V1.md` | CREATE |
| `docs/handoff/SEXNET_TCP_HANDSHAKE_GATE_V1.md` | CREATE |
| `docs/handoff/NETWORK_STACK_STATUS_ROLLUP_V1.md` | UPDATE |
| `scripts/daily_driver_master_gate.sh` | UPDATE: add sexnet_tcp_* gates |

