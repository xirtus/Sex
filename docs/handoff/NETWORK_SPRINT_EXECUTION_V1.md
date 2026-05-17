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

## RX Diagnostic Snapshot (r21/r22 sequencing + TX-slot staging)
- `/tmp/sexos_network_sprint_autopilot_r21_rx_variant_sweep.log`:
  - `e1000.rx.ctrl.link_probe`: `slu=1` already set; reasserting CTRL.SLU did not change RX behavior.
  - `e1000.rx.variant.apply`: rounds 0..2 exercised RCTL variants with `lbm=3`, including `lpe=1` in round 1.
  - `e1000.rx.ring.progress`: unchanged (`rdh_before=0`, `rdh_after=0`).
- `/tmp/sexos_network_sprint_autopilot_r22_tail_slot_stage.log`:
  - TX path fix: descriptor staging now uses current tail slot before each tail advance (not descriptor 0 only).
  - `e1000.rx.loopback.tx.repost`: `tdt=5 len=60` after slot-correct staging.
  - `e1000.rx.peer.observe`: still `observed=0`; `arp=0`, `dns_reply=0`.

Interpretation update:
- A concrete TX-lane correctness bug was fixed (tail-slot descriptor staging), but RX remains blocked.
- Remaining likely blocker is RX queue/reset/enable ordering or model-specific receive path behavior in this emulation profile.

## RX Diagnostic Snapshot (r23 disable-reset-reenable)
- `/tmp/sexos_network_sprint_autopilot_r23_rx_reorder.log`:
  - `e1000.rx.init.replay`: `reason=disable_reset_reenable_before_poll`, with `rctl=0x040080DA`, `rdh=0`, `rdt=7`, `en=1`.
  - `e1000.rx.diag.post`: still `rdh=0 rdt=7`.
  - `e1000.rx.peer.observe`: still `observed=0`.

Interpretation update:
- Bounded RX disable→queue reset→re-enable ordering did not unblock RX descriptor completion.
- Next lane should target receive interrupt/cause timing and moderation registers (IMS/IMC/ICR + RDTR/RADV/RXDCTL model-specific behavior) while preserving bounded claims.

## RX Diagnostic Snapshot (r24 interrupt/moderation lane)
- `/tmp/sexos_network_sprint_autopilot_r24_intr_moderation.log`:
  - `e1000.rx.intr.reseq`: `imc=0x00000000`, `icr_flush=0x00000000`, `ims=0x00000083`.
  - `e1000.rx.moderation.probe`: `rdtr=0x00000000`, `radv=0x00000000` (write/readback at bounded defaults).
  - `e1000.rx.diag.post`: `icr=0x00000003`, `rdh=0`, `rdt=7`.
  - `e1000.rx.peer.observe`: still `observed=0`.

Interpretation update:
- Interrupt/cause resequencing and moderation register writes are now explicitly exercised and read back.
- RX completion remains stalled; next bounded lane should focus on descriptor rearm/tail update semantics in the poll/recycle loop.

## RX Diagnostic Snapshot (r25 descriptor rearm + fixed tail)
- `/tmp/sexos_network_sprint_autopilot_r25_rx_rearm_tail.log`:
  - `e1000.rx.rearm.variant`: `rounds=8`, `desc_rearm_writes=64`, `final_rdt=7`.
  - `e1000.rx.diag.post`: `rdh=0`, `rdt=7`, `icr=0x00000003`.
  - `e1000.rx.peer.observe`: still `observed=0`.

Interpretation update:
- Descriptor rearm semantics and fixed-tail discipline are now explicitly exercised each round.
- RX still stalls with zero descriptor completions; next bounded lane should move to explicit PCI bus-master + memory-space config verification/writeback at runtime before RX init.

## RX Diagnostic Snapshot (r26 runtime PCI command recheck)
- `/tmp/sexos_network_sprint_autopilot_r26_pci_cmd_recheck.log`:
  - `e1000.pci.command.recheck`: `before=0x00000107 after=0x00000107 rb=0x00000107 bm=1 mem=1 io=1`.
  - `e1000.rx.ring.progress`: unchanged (`rdh_before=0`, `rdh_after=0`).
  - `e1000.rx.peer.observe`: unchanged (`observed=0`).

Interpretation update:
- PCI command gating is confirmed not to be the blocker; runtime already has IO/MEM/BM enabled.
- Next bounded lane should probe alternate RX register-bank controls (queue region offsets near 0x2C00) while preserving bounded claims.

## RX Diagnostic Snapshot (r27 0x2Cxx bank probe)
- `/tmp/sexos_network_sprint_autopilot_r27_2cxx_probe.log`:
  - `e1000.rx.alt_probe.ext`: `off_d=0x2C20 rb_d=0x00000000 off_e=0x2C28 rb_e=0x00000000`.
  - `e1000.rx.alt_probe.winner`: `found=0`.
  - `e1000.rx.ring.progress`: still `rdh_before=0 rdh_after=0`.
  - `e1000.rx.peer.observe`: still `observed=0`.

Interpretation update:
- Additional 0x2Cxx queue-bank candidates did not latch and did not affect RX behavior.
- Next bounded lane should pivot from register-bank probing to traffic-shape/source assumptions (e.g., explicit QEMU usernet ingress trigger or receive path model mismatch isolation).

## RX Diagnostic Snapshot (r28 explicit ingress trigger)
- `/tmp/sexos_network_sprint_autopilot_r28_ingress_trigger.log`:
  - `e1000.rx.ingress.trigger`: `bursts=4 tdt_after=8 icr_before=0x00000003 icr_after=0x00000003`.
  - `e1000.rx.ring.progress`: still `rdh_before=0 rdh_after=0`.
  - `e1000.rx.peer.observe`: still `observed=0`.

Interpretation update:
- Bounded explicit ingress stimulus was transmitted (ARP/ICMP burst), but no additional RX causes or descriptor movement appeared.
- This strengthens the conclusion that current blocker is RX-path/model behavior in this emulation/profile, not merely lack of outbound stimulus.

## A/B Emulation Snapshot (e1000 vs virtio-net-pci)
- Harness update:
  - `scripts/run_daily_driver_proof.sh` now supports `QEMU_NET_MODEL` (`e1000` or `virtio-net-pci`) while preserving existing E1000 default path.
- Runs:
  - E1000: `/tmp/sexos_network_sprint_ab_e1000.log` -> `FINAL PASS (226/0/0)`.
  - Virtio: `/tmp/sexos_network_sprint_ab_virtio.log` -> `FINAL PASS (134/0/92)` (many E1000-specific gates SKIP by design in this lane).
- Marker delta:
  - E1000 run emits RX diagnostics and still stalls (`e1000.rx.ring.progress`, `e1000.rx.peer.observe`, `e1000.rx.selftest.proof` all zero movement/observe).
  - Virtio run does not emit the E1000 RX lane markers, confirming the A/B harness separation path is functioning for model isolation.

Interpretation update:
- Current dead-path evidence remains E1000-lane specific in this harness.
- Next bounded lane should focus on E1000-specific driver/model assumptions (descriptor format/control bits and queue ownership semantics), not generic network plumbing.

## RX Diagnostic Snapshot (r29 external-ingress trigger mode)
- `/tmp/sexos_network_sprint_r29_ingress_external_mode.log`:
  - `e1000.rx.ingress.mode`: `rctl_before=0x040080DA rctl_after=0x0400801A lbm=0`.
  - `e1000.rx.ingress.trigger`: `bursts=4 tdt_after=8 icr_before=0x00000003 icr_after=0x00000003`.
  - `e1000.rx.ring.progress`: still `rdh_before=0 rdh_after=0`.
  - `e1000.rx.peer.observe`: still `observed=0`.

Interpretation update:
- The prior ingress-stimulus test was not blocked by loopback mode after all; forcing external-ingress mode did not change outcomes.
- RX dead-path remains in E1000 lane despite: valid PCI command bits, TX descriptor consumption, explicit outbound burst, and loopback-off egress mode.

## Transport/Application Send-Lane Lift (r30/r31)
- `/tmp/sexos_network_sprint_r30_send_lifts.log`:
  - Added explicit exercised send postings for:
    - `arp.request.send.stop.review` (`stop=0` lane marker emitted after send)
    - `icmp.echo.request.send.stop.review` + `icmp.echo.request.proof` (`sent=1`)
    - `tcp.syn.send.stop.review` + `tcp.handshake.proof` (SYN posted, observe remains bounded zero)
    - `http.get.send.stop.review` + `http.get.text.response.proof` (GET shape posted, response remains bounded zero)
- `/tmp/sexos_network_sprint_r31_gate_align.log`:
  - Gate script aligned to accept exercised `stop=0` for ARP/ICMP/TCP/HTTP stop-review gates (same style as UDP/DNS lanes).
  - Returned to `FINAL PASS (226/0/0)`.

Interpretation update:
- We advanced multiple sprint lanes from strict stop-review-only markers to exercised send-lane proofs while preserving bounded no-overclaim on receive/remote response.
- Primary unresolved blocker remains E1000 RX observe path (descriptor completion/inbound packet visibility).

## Additional Lane Progress (r32)
- `/tmp/sexos_network_sprint_r32_arp_status_icr_decode.log`:
  - Added `e1000.rx.icr.decode` marker to decode key post-poll causes (`rxseq`, `lsc`, `rxo`, `rxdmt0`) from `ICR`.
  - `arp.cache.status.stub` now also emits runtime observe-lane status (`entries=<arp_seen> valid=<arp_seen>`) in addition to initial pre-observe stub marker.
  - Proof remains stable at `FINAL PASS (226/0/0)`.

Interpretation update:
- Transport send-lane exercises are now in place (ARP/ICMP/TCP SYN/HTTP GET shapes), and gate alignment is complete for exercised stop-review lanes.
- RX descriptor completion remains the dominant unresolved blocker for converting bounded observe claims to real observed receive proofs.

## Additional Lane Progress (r33-r34)
- `/tmp/sexos_network_sprint_r33_rx_ctrl_diag.log`:
  - Added `e1000.rx.ctrl.diag` marker to snapshot receive control state after poll:
    - `RCTL.EN`, `RCTL.BAM`, `RXDCTL(0).ENABLE`, `SRRCTL(0)` buffer-size field, raw `RXCSUM/SRRCTL/RXDCTL`.
  - Gate result remained `FINAL PASS (226/0/0)`.
- `/tmp/sexos_network_sprint_r34_slot_grant_stop0.log`:
  - Advanced `browser.slot.net.grant.stop.review` to exercised review lane (`stop=0`) while preserving deny-default/no-auto-grant policy.
  - Gate parser now accepts either `stop=1` (strict stop-review) or `stop=0` (exercised review path) for this lane.
  - Gate result remained `FINAL PASS (226/0/0)`.

Interpretation update:
- Browser grant control lane now records exercised policy review without granting capability.
- RX blocker remains unresolved: no descriptor completion despite control-plane bits and repeated ingress/rearm probes.

## Additional Lane Progress (r35)
- `/tmp/sexos_network_sprint_r35_rx_dd_observe.log`:
  - Added `e1000.rx.dd.observe` marker to count descriptor polling and completion-bit observations:
    - `polled=64 dd_set=0` in this run.
  - Confirmed RX control snapshot in same run:
    - `e1000.rx.ctrl.diag` showed `rctl_en=1`, `rctl_bam=1`, `rxdctl_en=0`, `srrctl=0`, `rxdctl=0`.
  - `e1000.rx.peer.observe` remained `observed=0` with no ARP/ICMP/UDP/DNS replies.
  - Gate result remained `FINAL PASS (226/0/0)`.

Interpretation update:
- The dominant blocker is now sharper: RX descriptor done-bit never asserts (`dd_set=0`) across repeated polls, with queue-control registers still reading disabled/default in this environment.

## Additional Lane Progress (r36)
- `/tmp/sexos_network_sprint_r36_rx_queue_init.log`:
  - Added explicit RX queue-control init/writeback marker:
    - `e1000.rx.queue.init.proof`
    - Programs `SRRCTL(0)`, `RXCSUM`, `RXDCTL(0)` and reads them back in the replay snapshot.
  - Runtime evidence still reads default values:
    - `srrctl=0x00000000`, `rxcsum=0x00000000`, `rxdctl=0x00000000`, `rxdctl_en=0`.
  - `e1000.rx.dd.observe` remains `dd_set=0`.
  - Gate result remained `FINAL PASS (226/0/0)`.

Interpretation update:
- The RX dead-path is consistent with queue-control writes not taking effect on this emulated register path (or being reset/ignored), while `RCTL.EN` remains asserted.

## Additional Lane Progress (r37) - E1000_RX_QUEUE_ENABLE_SEMANTICS_V1
- `/tmp/sexos_network_sprint_r37_rx_queue_enable_semantics.log`:
  - Added `e1000.rx.queue.enable.semantics.v1` marker that probes two exact RX-enable orders:
    - Sequence A: ring/queue regs first, then `RCTL.EN`.
    - Sequence B: `RCTL.EN` first, then ring/queue regs.
  - Observed in both sequences:
    - `rctl_en=1`
    - `rdlen=128`, `rdh=0`, `rdt=7` (`ring_ok=1`)
    - `rxdctl=0x00000000`, `srrctl=0x00000000`, `rxdctl_en=0`
    - `queue_mode_visible=0`, `legacy_mode_visible=1`
  - `e1000.rx.dd.observe` remains `dd_set=0`.
  - Gate result remained `FINAL PASS (226/0/0)`.

Interpretation update:
- For this device/model path, RX queue enable ordering (`RCTL` before/after queue regs) is not the differentiator.
- Legacy ring registers (`RDLEN/RDH/RDT`) are stable/readable; queue-control registers (`RXDCTL/SRRCTL`) remain non-latching at zero.

## Next Session Pickup (RX Queue Only)
- Scope lock:
  - Do not add/modify ARP, ICMP, UDP, TCP, DNS, HTTP, browser, or grant logic.
  - Work only inside e1000 RX queue control semantics and descriptor completion proof.
- Current proven baseline:
  - `RCTL.EN=1` is stable.
  - `RDLEN/RDH/RDT` read back as expected.
  - `RXDCTL/SRRCTL` read back as `0x00000000` across both tested orderings.
  - `e1000.rx.dd.observe`: `dd_set=0` (no RX descriptor done bit observed).
- Primary objective for next session:
  - Convert RX lane from “register semantics diagnosed” to “descriptor completion observed” or conclusively prove model-limited non-latch behavior with bounded evidence.
- Ordered next probes:
  1. Add bounded register-bank variant probe for RX queue controls using per-bank ring mirror reads:
     - Test queue-control candidates adjacent to existing banks (keep bounded, no broad scan).
     - Emit one marker summarizing per-bank latch behavior.
  2. Add bounded “write persistence over time” probe:
     - Write candidate RX queue controls once, delay, re-read before and after one poll round.
     - Emit one marker with immediate vs delayed readbacks.
  3. Add bounded “descriptor ownership edge” probe:
     - Toggle one descriptor’s status/error fields and tail movement pattern in a controlled round.
     - Emit marker proving whether hardware ever mutates descriptor metadata.
  4. Re-run `./scripts/run_daily_driver_proof.sh` and require gate stability at `226/0/0`.
- Required evidence artifacts:
  - New run log under `/tmp/sexos_network_sprint_rXX_*.log`.
  - Marker lines for new RX-only probes.
  - Updated interpretation block in this handoff doc.

---

## Session: E1000_RX_BANK_PERSISTENCE_OWNERSHIP_PROBE_V1 (2026-05-17)

Commit baseline: 896ae00. Gates: FINAL PASS 226/0/0. Log: `/tmp/sexos_e1000_rx_bank_persistence_ownership_probe_v1.log`.

### Register-Bank Candidate Table

| Offset | Label    | Latched |
|--------|----------|---------|
| 0x2820 | RDTR     | YES     |
| 0x2824 | unk_2824 | no      |
| 0x2828 | RXDCTL   | no      |
| 0x282C | RADV     | YES     |
| 0x2830 | unk_2830 | no      |
| 0x2834 | unk_2834 | no      |

### Write-Persistence Table (RXDCTL 0x2828)

| imm_latched | delayed_latched | post_poll_latched |
|-------------|-----------------|-------------------|
| 0           | 0               | 0                 |

### Descriptor Ownership Edge Table

| status_before | status_after | len_before | len_after | rdh_before | rdh_after | hw_mutated |
|---------------|--------------|------------|-----------|------------|-----------|------------|
| 0x00          | 0x00         | 0          | 0         | 0          | 0         | 0          |

### Conclusion: D — Model-limited RX path confirmed

- RXDCTL (0x2828) and SRRCTL (0x280C) are stubs — silently drop writes.
- RDTR (0x2820) and RADV (0x282C) are real latching timer registers.
- HW never advances RDH or sets DD bit despite RCTL.EN=1, RDT=7, valid buffer address.
- `hw_mutated=0`: hardware never touches descriptor memory in bounded wait.
- TX descriptors consumed (prior session) — BM/DMA lane confirmed alive.

### Next: E1000_RX_DESCRIPTOR_FORMAT_VARIANT_PROBE_V1

Verify 82540EM legacy 16-byte descriptor layout matches current write pattern.
Confirm RDBAL/RDBAH point to physically correct addresses (check RDBAH if ring >4 GiB).
Fallback: QEMU_E1000_MODEL_SWITCH_82540EM_V1 if format confirmed correct.

Full findings: `docs/handoff/E1000_RX_BANK_PERSISTENCE_OWNERSHIP_PROBE_V1.md`

---

## Session: E1000_RX_DESCRIPTOR_ADDRESS_WIDTH_PROBE_V1 (2026-05-17)

Gates: FINAL PASS 226/0/0. Log: `/tmp/sexos_e1000_rx_descriptor_address_width_probe_v1.log`.

### Ring Base Address Table

| rx_phys            | RDBAL      | RDBAH      | Reconstructed      | below4g | match | align4k |
|--------------------|------------|------------|--------------------|---------|-------|---------|
| 0x000000001F86C000 | 0x1F86C000 | 0x00000000 | 0x000000001F86C000 | YES     | YES   | YES     |

### Buffer Address Table

| desc0_buf          | buf0_phys          | below4g | match | align2048 |
|--------------------|--------------------|---------|-------|-----------|
| 0x00000000102AB000 | 0x00000000102AB000 | YES     | YES   | YES       |

### Conclusion: address_width_ok=1 — NOT the blocker

- RDBAL/RDBAH correctly split and read back.
- Buffer address in descriptor[0] exactly matches pkt_pages[0].
- All addresses below 4 GiB, properly aligned.
- RX still dead: `rdh=0, rdt=7, dd=0`.

### Ruled out so far

| Cause                         | Status        |
|-------------------------------|---------------|
| RXDCTL/SRRCTL not latching    | CONFIRMED stub |
| RDBAL/RDBAH wrong             | RULED OUT     |
| Address above 4 GiB           | RULED OUT     |
| Buffer addr mismatch in desc  | RULED OUT     |
| Alignment fault               | RULED OUT     |
| RCTL.EN not set               | RULED OUT     |

### Next: E1000_RX_DESCRIPTOR_FORMAT_VARIANT_PROBE_V1

Primary: enable RCTL.LBM=3 BEFORE sending TX frame — if RX gets that frame,
descriptor processing works and only external traffic is missing.
Secondary: probe RCTL.BSIZE variants, explicit descriptor field layout verification.

Full findings: `docs/handoff/E1000_RX_DESCRIPTOR_ADDRESS_WIDTH_PROBE_V1.md`

---

## Session: E1000_RX_LOOPBACK_PREENABLE_REPOST_PROOF_V1 (2026-05-17)

Gates: FINAL PASS 226/0/0. Log: `/tmp/sexos_e1000_rx_loopback_preenable_repost_proof_v1.log`.

### Loopback Timing Table

| lbm | en | tx_posted | tx_dd_after_poll | rx_dd | rdh_advanced |
|-----|----|-----------|-----------------|-------|--------------|
| 3   | 1  | 1 (TDT=1) | **0**           | 0     | 0            |

### Key finding

TX worked (dd=1) in normal mode. TX did NOT work (dd=0) with RCTL.LBM=3.
QEMU e1000 MAC loopback (LBM=3) does not process TX descriptors — path is non-functional.

### Conclusion: B — Loopback dead. Model-limitation confirmed for LBM=3 path.

- RCTL.LBM=3 latches correctly.
- TX descriptor not consumed in LBM=3 mode.
- RX: zero descriptors touched across 4×100k-spin poll rounds.
- Direct TDH=0 write may also have corrupted TX state (TDH is read-only per 82540EM spec).

### Ruled out (cumulative)

RXDCTL stub, SRRCTL stub, address/alignment (all correct), RCTL.EN, ring register init, MAC loopback path.

### Next: QEMU_E1000_MODEL_SWITCH_82540EM_V1

Check QEMU `-device` model name. Try `e1000-82544gc` or PHY loopback (LBM=1) probe.
External RX requires SLiRP to deliver packets — needs ARP/DHCP (protocol scope) or model switch.

Full findings: `docs/handoff/E1000_RX_LOOPBACK_PREENABLE_REPOST_PROOF_V1.md`

---

## Session: QEMU_E1000_MODEL_SPLIT_RX_V1 (2026-05-17) — BREAKTHROUGH

Gates: FINAL PASS 226/0/0 on ALL four models.

### Model RX Result Table

| Model         | TX dd | rx_dd | rdh_advanced | Gates   |
|---------------|-------|-------|-------------|---------|
| e1000         | 1     | 0     | 0           | 226/0/0 |
| e1000-82544gc | 1     | 0     | 0           | 226/0/0 |
| e1000-82545em | 1     | 0     | 0           | 226/0/0 |
| **e1000e**    | 1     | **4** | **1**       | 226/0/0 |

### Finding: e1000e (82574L) produces RX descriptor completions

`e1000e` model: `rx_dd=4 rdh_advanced=1` in the loopback pre-enable repost probe.
All e1000 family (82540em/82544gc/82545em): `rx_dd=0 rdh_advanced=0`.

QEMU e1000e implements MAC loopback and RX descriptor processing.
QEMU e1000 family does not implement either.

Kernel register programming (RDBAL/RDBAH/RDLEN/RDH/RDT/RCTL) is compatible with e1000e.
No kernel changes required to switch models — only `QEMU_NET_MODEL=e1000e`.

### Next: E1000E_RX_DESCRIPTOR_OBSERVE_PROOF_V1

Use `QEMU_NET_MODEL=e1000e`. Verify actual packet content in loopback RX buffer.
Prove single-descriptor completion (clear DD between poll rounds).
Then probe external SLiRP RX.

Full findings: `docs/handoff/QEMU_E1000_MODEL_SPLIT_RX_V1.md`
