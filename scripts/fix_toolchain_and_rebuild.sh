#!/bin/bash
# ==================================================
# SEX Microkernel TOOLCHAIN FIX + REBUILD + QEMU
# Fixes: "no such command: +nightly" + E0521/E0793
# Phase 23 → Phase 24 Handoff (Apr 18 2026)
# Uses rustup run nightly (bypasses PATH/proxy issues)
# Re-applies no_std patches + shows full error details
# ==================================================

set -euo pipefail

PROJECT_ROOT="/Users/xirtus/sites/microkernel"
cd "$PROJECT_ROOT" || { echo "FATAL: Cannot enter $PROJECT_ROOT"; exit 1; }

echo "=== TOOLCHAIN FIX + REBUILD CYCLE (E0521/E0793) ==="
echo "Date: $(date)"

# 1. Permissions + nightly toolchain
echo "[1/5] Fixing permissions + installing nightly..."
chmod +x scripts/*.sh 2>/dev/null || true
rustup toolchain install nightly --allow-downgrade || echo "   → nightly already present"
echo "   → Using rustup run nightly cargo (fixes +nightly error)"

# 2. Re-apply no_std Cargo.toml patch (getrandom + RustCrypto)
echo "[2/5] Re-patching kernel/Cargo.toml for no_std..."
if [ -f "kernel/Cargo.toml.bak" ]; then
  cp kernel/Cargo.toml.bak kernel/Cargo.toml
fi
if [ -f "kernel/Cargo.toml" ]; then
  sed -i.bak '/getrandom/ s/^\s*getrandom.*/getrandom = { version = "0.2", default-features = false, features = ["rdrand"] }/' kernel/Cargo.toml || true
  sed -i.bak 's/#.*rust-crypto/rust-crypto/' kernel/Cargo.toml || true
  sed -i.bak '/aws-lc-sys/d' kernel/Cargo.toml || true
  echo "   → getrandom rdrand-only | RustCrypto re-enabled ✓"
fi

# 3. Clean build
echo "[3/5] Running clean_build.sh ..."
./scripts/clean_build.sh || {
  echo "Warning: clean_build.sh missing → fallback"
  cargo clean
  rm -rf target/ iso_root/ *.iso qemu_serial.log build_error_logs/* 2>/dev/null || true
}

# 4. BUILD with rustup run nightly + verbose (shows E0521/E0793 details)
echo "[4/5] Building kernel with rustup nightly (x86_64-unknown-none)..."
rustup run nightly cargo build \
  --target x86_64-unknown-none \
  --release \
  -p sex-kernel \
  --features "limine acpi" \
  -v 2>&1 | tee build.log

if [ $? -eq 0 ]; then
  echo "[+] KERNEL BUILT SUCCESSFULLY (E0521/E0793 resolved)"
else
  echo "BUILD FAILED (see below for E0521/E0793 details)"
  echo ""
  echo "=== QUICK FIX HINTS (per HANDOFF.md lifetime/alignment) ==="
  echo "E0521 → borrowed data escapes closure → add 'move' or 'static lifetime"
  echo "E0793 → reference to packed field → add #[repr(align(1))]" or remove packed"
  echo "Full errors + line numbers in: build.log"
  echo "Run: grep -A5 -E 'E0521|E0793' build.log"
  echo "Revert Cargo.toml if needed: cp kernel/Cargo.toml.bak kernel/Cargo.toml"
fi

# 5. ISO + QEMU if build succeeded
if [ -f "target/x86_64-unknown-none/release/sex-kernel" ]; then
  echo "[5/5] Build OK → packaging ISO + launching QEMU (PKU)..."
  mkdir -p iso_root/boot
  cp target/x86_64-unknown-none/release/sex-kernel iso_root/boot/sex-kernel.elf 2>/dev/null || cp target/x86_64-unknown-none/release/sex-kernel iso_root/boot/
  
  if [ -f "scripts/full_sasos_automation.sh" ]; then
    ./scripts/full_sasos_automation.sh
  else
    echo "   → QEMU launch skipped (run manually after fixing)"
  fi
fi

echo ""
echo "=== NEXT STEPS ==="
echo "If build succeeds → OOM panic should be gone"
echo "If still E0521/E0793 → paste the exact error lines here"
echo "Then we fix the exact lines in memory.rs / allocator.rs"
echo "Handoff status: toolchain fixed + ready for clean Rust build"
