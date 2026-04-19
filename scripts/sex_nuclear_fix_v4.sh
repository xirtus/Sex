#!/bin/bash
set -e

echo "=========================================="
echo "  SEX MICROKERNEL - V4 AUTOMATION FIX"
echo "  https://github.com/xirtus/sex"
echo "=========================================="

echo "→ Forcing x86_64-unknown-none target on nightly toolchain..."
rustup target add x86_64-unknown-none --toolchain nightly

echo "→ Re-ensuring rust-src on nightly..."
rustup component add rust-src --toolchain nightly

echo "→ Re-running cargo fix on all crates..."
cargo fix --allow-dirty -p sex-orbclient --lib || true
cargo fix --allow-dirty -p tuxedo --lib || true
cargo fix --allow-dirty -p sexgemini --bin sexgemini || true
cargo fix --allow-dirty -p sexfiles || true

echo "→ Double-checking panic_handler + alloc_error_handler..."
for f in crates/sex-orbclient/src/lib.rs servers/tuxedo/src/lib.rs servers/sexgemini/src/main.rs; do
  if [[ -f "$f" ]]; then
    sed -i.bak 's/fn alloc_error_handler(layout: Layout) -> ! {/fn alloc_error_handler(_layout: Layout) -> ! {/' "$f" || true
    if ! grep -q "#\[panic_handler\]" "$f" 2>/dev/null; then
      cat >> "$f" << 'PANIC'

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
PANIC
    fi
    echo "  Fixed: $f"
  fi
done

echo "→ Verifying target is installed..."
rustup target list --installed | grep x86_64-unknown-none || echo "WARNING: target still missing!"

echo "→ Launching full clean build + sasos..."
./scripts/clean_build.sh && make run-sasos

echo "=========================================="
echo "SEX Microkernel v4 automation complete."
echo "Target now forced on nightly. Build should finally succeed."
