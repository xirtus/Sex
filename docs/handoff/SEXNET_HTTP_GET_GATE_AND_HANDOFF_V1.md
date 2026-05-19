# SEXNET_HTTP_GET_GATE_AND_HANDOFF_V1

Added `sexnet_http_get_source3` gate to `scripts/daily_driver_master_gate.sh`.

PASS requires:
- `[sexnet.phaseI.stop_review.pass]`
- GET build done
- GET TX over ESTABLISHED with DD
- response RX done
- status parse done
- body buffer done
- `[sexnet.phaseI.readiness] ... source=3 ok=1`
- fault scan pass

SKIP:
- no ESTABLISHED / no payload TX / no peer response / env unavailable

FAIL:
- HTTP claimed without ESTABLISHED
- malformed status parse
- fault scan fails
