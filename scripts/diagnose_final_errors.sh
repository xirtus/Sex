#!/bin/bash
# SexOS SASOS - Final Error Isolation
set -euo pipefail

echo "--> REVEALING BLOCKERS IN SEXDISPLAY AND SEXGEMINI..."
grep -B 3 -A 10 "error\[" global_final.log | grep -v "warning:" || echo "No standard Rust errors found. Checking for linker errors..."

# If grep fails, look for 'linker' or 'panicked' at the end of the log
if ! grep -q "error\[" global_final.log; then
    tail -n 100 global_final.log | grep -E "error:|LD|linker|panic"
fi
