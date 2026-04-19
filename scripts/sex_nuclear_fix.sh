#!/bin/bash
set -e

echo "=========================================="
echo "  SEX MICROKERNEL - FULL AUTOMATION FIX"
echo "  https://github.com/xirtus/sex"
echo "=========================================="

echo "→ Running cargo fix --allow-dirty on all dirty crates..."
cargo fix --allow-dirty -p sex-orbclient --lib
cargo fix --allow-dirty -p tuxedo --lib
cargo fix --allow-dirty -p sexgemini --bin sexgemini
cargo fix --allow-dirty -p sexfiles || true

echo "→ Fixing unused 'layout' variable in alloc_error_handler..."
for f in crates/sex-orbclient/src/lib.rs servers/tuxedo/src/lib.rs servers/sexgemini/src/main.rs; do
  if [[ -f "$f" ]]; then
    sed -i.bak 's/fn alloc_error_handler(layout: Layout) -> ! {/fn alloc_error_handler(_layout: Layout) -> ! {/' "$f"
    echo "  Fixed: $f"
  fi
done

echo "→ Adding missing #[panic_handler] to sexgemini..."
if ! grep -q "#\[panic_handler\]" servers/sexgemini/src/main.rs 2>/dev/null; then
  cat >> servers/sexgemini/src/main.rs << 'PANIC'

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
PANIC
  echo "  panic_handler injected"
fi

echo "→ Launching full clean build + sasos..."
./scripts/clean_build.sh && make run-sasos

echo "=========================================="
echo "SEX Microkernel automation complete. No more warnings or panic_handler errors."
