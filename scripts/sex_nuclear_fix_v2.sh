#!/bin/bash
set -e

echo "=========================================="
echo "  SEX MICROKERNEL - V2 AUTOMATION FIX"
echo "  https://github.com/xirtus/sex"
echo "=========================================="

echo "→ Installing x86_64-unknown-none target (required for no_std)..."
rustup target add x86_64-unknown-none

echo "→ Re-running cargo fix on all crates..."
cargo fix --allow-dirty -p sex-orbclient --lib || true
cargo fix --allow-dirty -p tuxedo --lib || true
cargo fix --allow-dirty -p sexgemini --bin sexgemini || true
cargo fix --allow-dirty -p sexfiles || true

echo "→ Double-checking panic_handler and alloc_error_handler..."
for f in crates/sex-orbclient/src/lib.rs servers/tuxedo/src/lib.rs servers/sexgemini/src/main.rs; do
  if [[ -f "$f" ]]; then
    sed -i.bak 's/fn alloc_error_handler(layout: Layout) -> ! {/fn alloc_error_handler(_layout: Layout) -> ! {/' "$f"
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
echo "SEX Microkernel v2 automation complete."
echo "All targets installed, warnings fixed, should now build clean."
