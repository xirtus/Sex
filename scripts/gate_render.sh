#!/bin/sh
# gate_render.sh — Static gate for Silk DE render contract invariants.
set -e
FAIL=0

if rg -q "SILK_DE_BAR_ABI_V1" crates/silkbar-model/src/lib.rs 2>/dev/null; then
    echo "[gate_render] PASS: SILK_DE_BAR_ABI_V1 found"
else
    echo "[gate_render] FAIL: SILK_DE_BAR_ABI_V1 missing"
    FAIL=1
fi

if rg -q "fn validate_contract" crates/silkbar-model/src/lib.rs 2>/dev/null; then
    echo "[gate_render] PASS: validate_contract() found"
else
    echo "[gate_render] FAIL: validate_contract() missing"
    FAIL=1
fi

if rg -q "validate_silkbar_contract" servers/silkbar/src/main.rs 2>/dev/null; then
    echo "[gate_render] PASS: validate_silkbar_contract in silkbar"
else
    echo "[gate_render] FAIL: validate_silkbar_contract missing from silkbar"
    FAIL=1
fi

if rg -q "validate_silkbar_contract" servers/sexdisplay/src/main.rs 2>/dev/null; then
    echo "[gate_render] PASS: validate_silkbar_contract in sexdisplay"
else
    echo "[gate_render] FAIL: validate_silkbar_contract missing from sexdisplay"
    FAIL=1
fi

if rg -q "silk\.contract\.validate\.ok" servers/silkbar/src/main.rs 2>/dev/null &&
   rg -q "silk\.contract\.validate\.ok" servers/sexdisplay/src/main.rs 2>/dev/null; then
    echo "[gate_render] PASS: contract validate markers in both"
else
    echo "[gate_render] FAIL: contract validate markers missing"
    FAIL=1
fi

if rg -q "silk\.render_proof\.top_strip\.ok" servers/sexdisplay/src/main.rs 2>/dev/null; then
    echo "[gate_render] PASS: top_strip render proof found"
else
    echo "[gate_render] FAIL: top_strip render proof missing"
    FAIL=1
fi

if [ $FAIL -eq 0 ]; then
    echo "[gate_render] ALL CHECKS PASSED"
else
    echo "[gate_render] SOME CHECKS FAILED"
fi
exit $FAIL
