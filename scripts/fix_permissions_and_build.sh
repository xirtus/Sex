#!/bin/bash
# ==================================================
# SEX Microkernel PERMISSION FIX + BUILD + QEMU
# Fixes "Permission denied" on full_sasos_automation.sh
# Then runs full clean → Rust kernel → ISO → QEMU test
# Phase 23 → Phase 24 (Apr 18 2026)
# ==================================================

set -euo pipefail

PROJECT_ROOT="/Users/xirtus/sites/microkernel"
cd "$PROJECT_ROOT" || { echo "FATAL: Cannot enter $PROJECT_ROOT"; exit 1; }

echo "=== FIXING PERMISSIONS + RE-RUNNING BUILD + QEMU ==="
echo "Date: $(date)"

# Fix execute bits on ALL our automation scripts
echo "[1/3] chmod +x on every .sh script..."
chmod +x scripts/*.sh 2>/dev/null || true
chmod +x full_sasos_automation.sh build_and_test_qemu.sh rustify_confirm.sh 2>/dev/null || true
echo "   → Permissions fixed ✓"

# 2. Clean first (style preference)
echo "[2/3] Running clean_build.sh ..."
./scripts/clean_build.sh || {
  echo "Warning: clean_build.sh missing → fallback clean"
  cargo clean
  rm -rf target/ iso_root/ *.iso qemu_serial.log 2>/dev/null || true
}

# 3. Full automation (build + ISO + QEMU)
echo "[3/3] Launching full_sasos_automation.sh → kernel build + Limine ISO + PKU QEMU..."
./scripts/full_sasos_automation.sh

echo ""
echo "=== BUILD + QEMU CYCLE COMPLETE ==="
echo "Check qemu_serial.log for 'OOM PANIC RESOLVED ✓'"
echo "If clean → kernel is now 100% Rust no_std + crypto re-enabled"
echo "Next step (tell me the QEMU output): rustify remaining C servers"
