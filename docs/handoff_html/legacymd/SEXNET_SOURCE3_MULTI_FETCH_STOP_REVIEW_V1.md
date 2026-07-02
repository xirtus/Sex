# SEXNET_SOURCE3_MULTI_FETCH_STOP_REVIEW_V1

Date: 2026-05-19
Branch: master
Task: 61 — Phase M stop review for source3 multi-fetch

## Review Questions

### 1. Does current source3 HTTP proof run once or multiple times?
**Answer: Once.** The current source3 HTTP GET path runs exactly once per boot, inside the `if is_established == 1 && payload_tx_sent == 1` block. After TX+RX+parse completes, control falls through to the PDX dispatch loop. No repeat loop exists.

### 2. Can repeated fetch be done without new protocol features?
**Answer: Yes.** The existing TCP SYN→SYN-ACK→final ACK path and HTTP GET TX+RX+parse path already contain all protocol primitives needed. Repeated fetch requires:
- TCP state reset to Closed
- Fresh TCP SYN handshake
- HTTP GET over ESTABLISHED
- RX response parse
No new protocol features. No DNS. No TLS. No browser raw NIC.

### 3. Can the host peer accept multiple connections?
**Answer: Yes.** The Phase M Python HTTP peer binds with SO_REUSEADDR and loops on accept(). Each client connection gets a fresh socket with sendall(RESP) then close(). Multiple sequential connections from the same guest IP:port pair work because the guest uses the same local port for each iteration and the server's previous connection socket is fully closed.

### 4. Are TX descriptors reused safely?
**Answer: Yes.** The existing path uses TX descriptor slot 7 (index 7 in the ring), publishing TDT=8 (wraps to 0 on 8-entry ring). Before each TX:
- Frame buffer is overwritten with new ETH+IPv4+TCP headers
- DD bit is cleared by hardware after readback
- Poll waits for DD=1
Reuse is safe because each TX is a complete atomic operation.

### 5. Are RX descriptors reused/cleared safely?
**Answer: Yes.** The existing RX path scans all 8 descriptors, clears status byte and length, and writes RDT tail to release descriptors back to hardware. This cleanup happens on each RX poll cycle. Reuse across iterations is safe because descriptors are fully recycled before the next iteration's RX poll begins.

### 6. Are retry/timeouts bounded?
**Answer: Yes.** All poll loops have hard iteration caps:
- TX DD poll: 50,000,000 iterations max
- RX poll: 1,000,000 iterations max
- No infinite loops
- No unbounded waits
- TCP handshake timeout: if SYN-ACK not received within poll window, state transitions to FAILED_TIMEOUT

### 7. Can browser render update multiple times without display ownership changes?
**Answer: Yes.** The browser's `shell_draw_text` route passes through the existing sexdisplay framebuffer writer. No display ownership changes are needed between renders. The FB bounds checks are stateless. Multiple calls to `shell_draw_text` with different content are safe and proven by the existing V1 browser proof path.

### 8. Can Phase M complete without DNS/TLS/browser raw NIC?
**Answer: Yes.** Phase M targets reliability only:
- No DNS (IP is hardcoded 10.0.2.2:18081)
- No TLS (plain HTTP)
- No browser raw NIC (never allowed)
- No HAL source2 revival (frozen as legacy)
- No kernel/ABI/scheduler changes

### 9. What STOP FIRST boundaries apply?
**STOP FIRST required before:**
- kernel edits
- ABI edits (sex-pdx)
- scheduler/time changes
- NIC ownership model changes
- source3 DNS implementation
- HAL deletion
- browser raw NIC access
- broad HTTP/TCP rewrite
- unbounded retry loops
- accepting partial/malformed HTTP as success

**No STOP FIRST needed for:**
- Adding bounded multi-fetch loop in sexnet server
- Adding Phase M markers to sexnet and silk-shell
- Adding Phase M gates to daily_driver_master_gate.sh
- Adding Phase M profile to run_daily_driver_proof.sh
- Creating handoff docs

## STOP REVIEW RESULT

**[sexnet.phaseM.multi_fetch.stop_review.pass]**

Phase M multi-fetch can proceed safely. All STOP FIRST boundaries are respected. No kernel, ABI, scheduler, or protocol redesign is needed. The implementation adds a bounded loop within the existing sexnet server that reuses the proven TCP handshake and HTTP GET primitives.
