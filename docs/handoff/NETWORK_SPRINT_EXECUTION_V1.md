# NETWORK_SPRINT_EXECUTION_V1

## Scope
Execution tracker for the network/browser sprint chain from E1000 bring-up to browser remote fetch and freeze handoff.

## Bundle Status

### Bundle A: E1000 runtime bring-up (implemented in this pass)
- `E1000_DESCRIPTOR_READBACK_PROOF_V1` (already present baseline)
- `E1000_MMIO_RING_BASE_WRITE_PLAN_V1` (implemented marker)
- `E1000_MMIO_RING_BASE_PROOF_V1` (implemented marker + gate)
- `E1000_RX_REGISTER_INIT_PLAN_V1` (implemented marker)
- `E1000_RX_REGISTER_INIT_PROOF_V1` (implemented marker + gate)
- `E1000_RX_ENABLE_STOP_REVIEW_V1` (implemented marker)
- `E1000_RX_ENABLE_PROOF_V1` (implemented marker + gate)
- `E1000_TX_REGISTER_INIT_PLAN_V1` (implemented marker)
- `E1000_TX_REGISTER_INIT_PROOF_V1` (implemented marker + gate)
- `E1000_TX_PACKET_STOP_REVIEW_V1` (implemented marker)
- `E1000_TX_TEST_FRAME_PLAN_V1` (implemented marker)
- `E1000_TX_TEST_FRAME_PROOF_V1` (implemented marker + gate)
- `E1000_RX_PACKET_OBSERVE_PROOF_V1` (implemented bounded claim + gate)

### Bundle B: Ethernet/ARP/IPv4/ICMP (in progress: marker/stub lane implemented)
- `ETHERNET_FRAME_MODEL_SPEC_V1`
- `ARP_CLIENT_PLAN_V1`
- `ARP_REQUEST_BUILD_PROOF_V1`
- `ARP_REQUEST_SEND_STOP_REVIEW_V1`
- `ARP_REQUEST_SEND_PROOF_V1`
- `ARP_REPLY_OBSERVE_PROOF_V1`
- `ARP_CACHE_STATUS_STUB_V1`
- `IPV4_PACKET_MODEL_SPEC_V1`
- `IPV4_HEADER_BUILD_PROOF_V1`
- `ICMP_ECHO_REQUEST_PLAN_V1`
- `ICMP_ECHO_REQUEST_SEND_STOP_REVIEW_V1`
- `ICMP_ECHO_REQUEST_PROOF_V1`
- `ICMP_ECHO_REPLY_OBSERVE_PROOF_V1`

Status note:
- Implemented bounded proof markers on the E1000 TX local path (no external peer-claim).
- ARP/ICMP send stages are currently explicit stop-review markers (`stop=1`) until dedicated Ethertype-specific frame lanes are staged.

### Bundle C: UDP/TCP transport (in progress: marker/stub lane implemented)
- `UDP_PACKET_MODEL_SPEC_V1`
- `UDP_TX_BUILD_PROOF_V1`
- `UDP_TX_SEND_STOP_REVIEW_V1`
- `UDP_LOOPBACK_OR_QEMU_USERNET_PROOF_V1`
- `TCP_MINIMAL_STATE_MACHINE_PLAN_V1`
- `TCP_SYN_BUILD_PROOF_V1`
- `TCP_SYN_SEND_STOP_REVIEW_V1`
- `TCP_HANDSHAKE_PROOF_V1`

Status note:
- Implemented bounded model/build/observe markers.
- TCP send/handshake remain explicit stop-review/no-peer-observe in this phase.

### Bundle D: DNS/HTTP core client (in progress: marker/stub lane implemented)
- `DNS_CLIENT_PLAN_V1`
- `DNS_QUERY_BUILD_PROOF_V1`
- `DNS_QUERY_SEND_STOP_REVIEW_V1`
- `DNS_RESPONSE_PARSE_PROOF_V1`
- `DNS_TO_HTTP_HOST_RESOLUTION_PROOF_V1`
- `HTTP_TEXT_FETCH_GRANT_PLAN_V1`
- `HTTP_GET_SEND_PLAN_V1`
- `HTTP_GET_SEND_STOP_REVIEW_V1`
- `HTTP_GET_TEXT_RESPONSE_PROOF_V1`
- `HTTP_RESPONSE_BOUNDED_BUFFER_PROOF_V1`
- `HTTP_404_AND_ERROR_PAGE_PROOF_V1`

Status note:
- Implemented DNS/HTTP plan/build marker chain with bounded no-network claims.
- DNS send and HTTP GET send remain explicit stop-review markers until transport lane is wired.

### Bundle E: Collar/browser network integration (in progress: marker/stub lane implemented)
- `BROWSER_HTTP_FETCH_GRANT_PLAN_V1`
- `COLLAR_BROWSER_NETWORK_GRANT_PLAN_V1`
- `COLLAR_BROWSER_NETWORK_GRANT_STUB_V1`
- `BROWSER_SLOT_NET_GRANT_STOP_REVIEW_V1`
- `BROWSER_SLOT_NET_GRANT_PROOF_V1`
- `HTTP_RESPONSE_TO_HTML_SUBSET_FEED_V1`
- `BROWSER_REMOTE_TEXT_RENDER_PROOF_V1`
- `BROWSER_FETCH_STATUS_UI_V1`
- `BROWSER_LINK_FETCH_GATED_PROOF_V1`
- `BROWSER_HISTORY_REMOTE_ENTRY_PROOF_V1`
- `BROWSER_TAB_REMOTE_STATUS_PROOF_V1`

Status note:
- Implemented grant/fetch/UI markers with explicit `granted=0` / `fetched=0` bounded claims.
- Runtime grant activation remains stop-reviewed in this phase.

### Bundle F: resilience/UX/dashboard/TLS-deferred (in progress: marker/stub lane implemented)
- `NETWORK_FAULT_CONTAINMENT_PROOF_V1`
- `NETWORK_TIMEOUT_AND_RETRY_POLICY_V1`
- `TLS_DEFERRED_TRUTH_SPEC_V1`
- `BROWSER_NO_TLS_WARNING_UI_V1`
- `BROWSER_HTTP_ONLY_FETCH_PROOF_V1`
- `BROWSER_USABILITY_KEYBOARD_NAV_V1`
- `BROWSER_URL_BAR_EDIT_PROOF_V1`
- `BROWSER_ENTER_TO_FETCH_GATED_PROOF_V1`
- `BROWSER_BACK_FORWARD_REMOTE_HISTORY_V1`
- `BROWSER_RELOAD_STOP_PROOF_V1`
- `SEXNET_STATUS_DASHBOARD_V1`
- `MESH_NETWORK_ROUTE_VISUAL_STUB_V1`
- `COLLAR_NETWORK_GRANT_UI_SPEC_V1`
- `COLLAR_NETWORK_GRANT_UI_STUB_V1`

Status note:
- Implemented policy/spec/status markers for fault containment, timeout/retry, TLS deferred truth, keyboard UX, and dashboard stubs.
- Runtime remote fetch and interactive UI transitions still require later live-lane proof.

### Bundle G: hardware/fallback/freeze (in progress: marker/plan lane implemented)
- `REAL_HARDWARE_NIC_AUDIT_V1`
- `REAL_HARDWARE_E1000_FALLBACK_PLAN_V1`
- `RUNTIME_SMOKE_REAL_NETWORK_PIPELINE_V1`
- `DAILY_DRIVER_NETWORK_BASELINE_FREEZE_V1`
- `NETWORK_SPRINT_FINAL_RUNTIME_SMOKE_V1`
- `NETWORK_SPRINT_HANDOFF_FREEZE_V1`

Status note:
- Added explicit markers for real-hardware audit/fallback/smoke/freeze as pending lanes (`pass=0` or `done=0`).

## File Changes in This Pass
- `kernel/src/hal/pci.rs`
- `scripts/daily_driver_master_gate.sh`

## Proof Notes
- TX/RX packet claims remain bounded: local descriptor/ring/register evidence only; no external peer delivery claim in this pass.
- Runtime compile check in this shell is blocked by missing local target (`x86_64-sex`).
- Runtime proof evidence (QEMU headless):
  - `/tmp/sexos_network_sprint_autopilot.log` first run: marker coverage PASS but `faults_zero` false-positive due marker token (`panic=0`).
  - `/tmp/sexos_network_sprint_autopilot_r2.log` rerun after marker fix: **FINAL PASS (224 gates, 0 fail, 0 skip)**.
  - `/tmp/sexos_network_sprint_autopilot_r18_alt_probe.log` after RX diagnostics + alt-latch probe: **FINAL PASS (226 gates, 0 fail, 0 skip)**.

## RX Diagnostic Snapshot (r18)
- `e1000.tx.consume.diag`: `desc0_status=0x01 dd=1` (TX descriptor consumed by device).
- `e1000.rx.ring.progress`: `rdh_before=0 rdt_before=7 rdh_after=0 rdt_after=7` (no RX ring advance).
- `e1000.rx.peer.observe`: `observed=0 arp=0 icmp_reply=0 udp=0 dns_reply=0`.
- `e1000.rx.replay.order`: `rxdctl=0x00000000` despite replayed init writes.
- `e1000.rx.alt_probe`: tested `0x2828`, `0x108`, `0x210`; all read back `0x00000000`.
- `e1000.rx.alt_probe.winner`: `found=0` (no alternate control-latch candidate in bounded set).

Interpretation:
- Current QEMU/usernet lane proves descriptor/ring/programming markers and TX consumption.
- RX data-path still does not advance in this bounded probe set; next step is register-map variant narrowing or model-specific RX control sequencing while keeping claims bounded.

## RX Diagnostic Snapshot (r19/r20 loopback lane)
- `/tmp/sexos_network_sprint_autopilot_r19_loopback.log`:
  - `e1000.rx.loopback.mode`: `lbm=3 en=1` latched in RCTL.
  - `e1000.rx.selftest.proof`: `observed=0 loopback=0`.
- `/tmp/sexos_network_sprint_autopilot_r20_loopback_repost.log`:
  - `e1000.rx.loopback.tx.repost`: `tdt=5 len=60` (TX repost after loopback enable).
  - `e1000.rx.ring.progress`: `rdh_before=0 rdh_after=0`.
  - `e1000.rx.peer.observe`: `observed=0 arp=0 icmp_reply=0 udp=0 dns_reply=0`.

Interpretation update:
- Loopback-mode enable plus post-enable TX repost did not produce RX descriptor completion in this emulation/profile.
- Next bounded lane should focus on queue/register model mismatch (RXDCTL/register bank variant) rather than more packet-shape changes.
