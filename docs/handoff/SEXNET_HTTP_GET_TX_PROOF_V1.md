# SEXNET_HTTP_GET_TX_PROOF_V1

Implemented GET TX on existing ESTABLISHED TCP payload path only.

Guard:
- TX allowed only when state is ESTABLISHED.

Proof markers:
- `[sexnet.http.get.tx.guard] state=ESTABLISHED ok=1`
- `[sexnet.http.get.tx.psh_ack] payload_len=N tx_dd=1 ok=1`
- `[sexnet.http.get.tx.proof.done] sent=1 tx_dd=1 ok=1`
