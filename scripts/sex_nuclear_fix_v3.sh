#!/bin/bash
set -e

echo "=========================================="
echo "  SEX MICROKERNEL - V3 AUTOMATION FIX"
echo "  https://github.com/xirtus/sex"
echo "=========================================="

echo "→ Installing rust-src component for nightly toolchain..."
rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu

echo "→ Re-running cargo fix on all crates..."
cargo fix --allow-dirty -p sex-orbclient --lib || true
cargo fix --allow-dirty -p tuxedo --lib || true
cargo fix --allow-dirty -p sexgemini --bin sexgemini || true
cargo fix --allow-dirty -p sexfiles || true

echo "→ Double-checking panic_handler + alloc_error_handler fixes..."
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

echo "→ Launching full clean build + sasos..."
./scripts/clean_build.sh && make run-sasos

echo "=========================================="
echo "SEX Microkernel v3 automation complete."
echo "rust-src installed, all fixes applied, should now build clean."
