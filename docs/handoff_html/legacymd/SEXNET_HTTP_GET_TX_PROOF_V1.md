# SEXNET_HTTP_GET_TX_PROOF_V1

Implemented GET TX on existing ESTABLISHED TCP payload path only.

Guard:
- TX allowed only when state is ESTABLISHED.

Proof markers:
- `[sexnet.http.get.tx.guard] state=ESTABLISHED ok=1`
- `[sexnet.http.get.tx.psh_ack] payload_len=N tx_dd=1 ok=1`
- `[sexnet.http.get.tx.proof.done] sent=1 tx_dd=1 ok=1`

## 2026-05-19 Update
- HTTP GET TX still rides the ESTABLISHED guard.
- Added PSH/ACK wire-shape diagnostics at send-time plus peer-ACK progression marker.
- Current proof attempt in `/tmp/sexnet_tcp_psh_ack_wireshape_fix.log` stayed in `no_network_grant_no_route`, so GET source3 runtime proof is not re-established from that run.
