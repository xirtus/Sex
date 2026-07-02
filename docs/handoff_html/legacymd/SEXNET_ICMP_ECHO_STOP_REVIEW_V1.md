# SEXNET_ICMP_ECHO_STOP_REVIEW_V1

Date: 2026-05-19
Branch: master
Commit: pending (Phase D stop review)
Review: STOP review for ICMP echo reply path before Phase D implementation

## Review Questions

### 1. Where does validated IPv4 proto=1 dispatch currently stop?

Currently the IPv4 path (`sexnet.ipv4.rx.poll` in `servers/sexnet/src/main.rs:~1904-2128`)
parses the IPv4 header, validates version/ihl/length/fragmentation/dst/checksum, and
reads the `proto` field — but then just logs `[sexnet.ipv4.rx.validate]` with
`proto=N` and stops. There is no dispatch on `proto==1`.

After logging the validation marker, the code recycles the RX descriptor and
exits the poll loop after 1 frame. No ICMP/UDP/TCP processing exists.

### 2. Is there already ICMP echo request parsing?

No. The `proto` field is read from the IPv4 header at offset 23 (line ~1973)
and logged, but there is no ICMP header parse, no type/code/checksum extract,
and no echo request identification.

### 3. Is there already ICMP checksum code?

No. The only checksum code is the IPv4 header checksum validation in the RX path
(lines ~2027-2048). ICMP checksum computation will need to be added.

### 4. Is there already ICMP echo reply TX code?

No. There is no ICMP TX path of any kind.

### 5. Can ICMP reply reuse existing Ethernet/IPv4 TX descriptor path without driver redesign?

Yes. The existing TX infrastructure is well-proven:
- `TX_PERM_FRAME_VA` — shared TX frame buffer (already used by ARP reply and L2 reuse)
- `TX_PERM_DESC_VA` — TX descriptor ring base (descriptors 0,1,2 already used; descriptor 3 is free)
- `TX_PERM_FRAME_PHYS` — physical address of TX frame buffer
- `nic_va + 0x3818` — TDT (Transmit Descriptor Tail) register
- DD-bit polling pattern is established (50M iteration bounded loop)

The ICMP echo reply needs only:
1. Write Ethernet+IPv4+ICMP headers and payload into TX_PERM_FRAME_VA
2. Set up descriptor at index 3 (TX_PERM_DESC_VA + 48)
3. Post TDT=4
4. Poll DD bit on descriptor 3

No driver redesign. No new BAR mapping. No new descriptor ring allocation.

### 6. Can host ping observe be done in this environment without forbidden root/raw socket requirements?

On TAP backend with root access: yes. `ping -I tap0 -c 1 -W 1 10.0.2.15` works
if run as root (or with CAP_NET_RAW). This is the same privilege model used by
the existing ARP host observe probe (`scripts/host_arp_reply_observe_probe.sh`).

If root/CAP_NET_RAW is unavailable: the host ping observe gate SKIPs honestly
with environment reason. Guest-side ICMP RX+TX proof still stands independently.

On usernet backend: ping cannot reach the guest NIC, so host observe SKIPs.

### 7. Can Phase D complete without UDP/DNS/TCP/HTTP?

Yes. Phase D scope is strictly:
- ICMP echo request parse (type=8, code=0)
- ICMP echo reply build and TX
- No UDP, no DNS, no TCP, no HTTP, no browser networking

The IPv4 path already parses `proto` — Phase D only adds the `proto==1` branch.

### 8. What STOP FIRST boundaries apply?

All STOP FIRST boundaries are respected:
- No kernel edits           ✅ (sexnet server only)
- No sex-pdx/global ABI edits ✅
- No MPK/PKU/PKEY changes    ✅
- No scheduler/PKRU/time     ✅
- No browser/sexdisplay/shell ✅
- No UDP/DNS/TCP/HTTP        ✅
- No routing table            ✅
- No ARP cache redesign       ✅
- No HAL NET_DIAG retirement  ✅
- No unbounded loops          ✅ (50M iteration TX DD poll, same pattern as ARP/L2)
- No fake PASS                ✅

## ICMP Echo Contract

- Only handle IPv4 protocol=1
- Only handle ICMP type=8 code=0 echo request
- Validate ICMP length >= 8 bytes (header minimum)
- Validate total_len from IPv4 bounds before reading ICMP body
- Preserve identifier and sequence number in echo reply
- Preserve payload up to bounded frame length
- Reply type=0 code=0
- Compute ICMP checksum over ICMP header + payload
- Build IPv4 reply:
  - src = 10.0.2.15 (SEXNET_GUEST_IPV4)
  - dst = request src
  - protocol=1
  - ttl=64
  - total_len = 20 (IPv4) + ICMP total length
  - valid IPv4 header checksum
- Build Ethernet reply:
  - dst MAC = request source MAC (Ethernet header bytes 6-11)
  - src MAC = NIC MAC (from RAL/RAH)
  - ethertype=0x0800
- TX descriptor done must be observed (DD bit poll)
- No routing
- No fragmentation support (fragmented IPv4 already rejected in Phase C)
- No UDP/TCP dispatch
- No browser path

## Existing Gates Declared (pre-Phase D)

Gate declarations already exist in `scripts/daily_driver_master_gate.sh`:
- `gate_icmp_echo_request_plan` (line 230, default SKIP)
- `gate_icmp_echo_request_send_stop_review` (line 231, default SKIP)
- `gate_icmp_echo_request_proof` (line 232, default SKIP)
- `gate_icmp_echo_reply_observe_proof` (line 233, default SKIP)

These are pre-existing marker-only gates that check for markers emitted by
higher-level stubs. Phase D adds the actual ICMP echo reply runtime path and
will add proper gates (`sexnet_icmp_echo_reply`, `sexnet_icmp_host_ping_observe`).

The old gates will remain SKIP if their expected markers are absent; Phase D
will not rename or break them.

## STOP Review Conclusion

All review questions indicate Phase D is safe to proceed:
- No forbidden edits required
- Existing TX descriptor path is proven and reusable
- ICMP echo reply is a narrow addition to the validated IPv4 RX path
- All STOP FIRST boundaries respected
- Host ping observe is optional and can SKIP honestly

**Verdict: PASS REVIEW — implementation can proceed.**

## Markers

- [sexnet.phaseD.stop_review.pass]
