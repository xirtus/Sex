#!/usr/bin/env bash
set -euo pipefail

echo "[AGENT PREFLIGHT]"
echo "1. Read docs/AGENT_README_FIRST.md"
echo "2. Read docs/AGENT_HANDOFF_GP_CLOCK.md"
echo "3. Read docs/COMMON_FAILURES.md"
echo "4. Use QUICKMAP docs. Do not open huge files fully."
echo
echo "[STATUS]"
git status --short
echo
echo "[INTERRUPTS LANDMARKS]"
rg "page_fault_handler|timer_interrupt|switch_to|faulted_task_halt|page_fault_stub|general_protection|send_eoi" kernel/src/interrupts.rs -n || true
