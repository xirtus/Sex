#!/bin/sh
# gate_no_alpha.sh — Enforce flat ARGB/top-strip safe subset.
# Fail if forbidden alpha/blur/shadow implementation symbols found.
set -e
FAIL=0

FORBIDDEN="alpha_blend|blend_alpha|blur_pass|shadow_pass|translucency|compositing_pass|full_frame_effect|gaussian|frosted"

for f in servers/sexdisplay/src/main.rs crates/silkbar-model/src/lib.rs; do
    if [ ! -f "$f" ]; then
        echo "[gate_no_alpha] SKIP: $f not found"
        continue
    fi
    MATCHES=$(rg -n "$FORBIDDEN" "$f" 2>/dev/null || true)
    if [ -n "$MATCHES" ]; then
        # Filter out comment-only lines that say forbidden/no alpha/no blur/no shadow
        CLEAN=$(echo "$MATCHES" | rg -v "^\s*//.*forbidden|no alpha|no blur|no shadow|no translucency" 2>/dev/null || true)
        if [ -n "$CLEAN" ]; then
            echo "[gate_no_alpha] FAIL: Forbidden symbols in $f:"
            echo "$CLEAN"
            FAIL=1
        else
            echo "[gate_no_alpha] PASS: $f (only safe doc comments)"
        fi
    else
        echo "[gate_no_alpha] PASS: $f clean"
    fi
done

if [ $FAIL -eq 0 ]; then
    echo "[gate_no_alpha] ALL CHECKS PASSED"
else
    echo "[gate_no_alpha] SOME CHECKS FAILED"
fi
exit $FAIL
