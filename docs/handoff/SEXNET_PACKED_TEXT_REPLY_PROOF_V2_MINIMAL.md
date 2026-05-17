# SEXNET_PACKED_TEXT_REPLY_PROOF_V2_MINIMAL

## Result
STOP

## What was implemented
- `servers/sexnet/src/main.rs`
  - Added local constants:
    - `SEXNET_HTTP_PROOF_LEN = 0x207`
    - `SEXNET_HTTP_PROOF_CHUNK = 0x208`
    - `PROOF_TEXT = b"HTTP 200 from 10.0.2.2"`
  - Added packed-text handlers on the existing `SEXNET_GET_STATUS` route:
    - `arg0 == 0x207` returns text length and logs `[sexnet.packed_text.len]`
    - `arg0 == 0x208` uses `arg1=chunk_idx`, packs up to 8 bytes little-endian, logs `[sexnet.packed_text.chunk]`

- `apps/kaleidoscope/src/main.rs`
  - Kept scalar route proof marker flow.
  - Added bounded packed-text fetch using local `[u8;64]`, max 8 chunks.
  - Fetch strategy:
    - `pdx_call(SLOT_NET, 0x200, 0x207, 0, 0)` for LEN
    - `pdx_call(SLOT_NET, 0x200, 0x208, idx, 0)` for CHUNK
  - Emits:
    - `[browser.packed_text.begin]`
    - `[browser.packed_text.len.recv] ...`
    - `[browser.packed_text.chunk.recv] ...`
    - `[browser.packed_text.text.set] ...`
    - `[browser.packed_text.proof.done]`

## Runtime outcome
- Build passes.
- Browser scalar marker still present: `[browser.slot.net.route.call] status=0`.
- Browser packed-text markers emit, but LEN is `0`:
  - `[browser.packed_text.len.recv] len=0`
- No `sexnet.packed_text.*` markers appear in runtime log.

## Blocking reason
Packed-text opcodes are not reaching/triggering sexnet packed-text branches in this runtime path, despite successful SLOT_NET scalar status call. Mission PASS criteria requiring sexnet chunk emit/recv markers cannot be met without clarifying the exact slot-opcode transport semantics for this lane.
