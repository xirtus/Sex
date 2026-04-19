#!/bin/bash
set -e

echo "=========================================="
echo "  SEX MICROKERNEL - V6 AUTOMATION FIX"
echo "  https://github.com/xirtus/sex"
echo "=========================================="

echo "→ Setting nightly override..."
rustup override set nightly

echo "→ Installing x86_64-unknown-none on nightly..."
rustup target add x86_64-unknown-none --toolchain nightly

echo "→ Installing rust-src on nightly..."
rustup component add rust-src --toolchain nightly

echo "→ Creating robust .cargo/config.toml..."
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[build]
target = "x86_64-unknown-none"
rustflags = ["-C", "target-cpu=generic"]

[unstable]
build-std = ["core", "alloc", "compiler_builtins"]
EOF
echo "  .cargo/config.toml created"

echo "→ Full cargo clean (wipes old host-target artifacts)..."
cargo clean

echo "→ Re-running cargo fix..."
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

echo "→ Verifying active target..."
rustup show active-toolchain
echo "Installed targets:"
rustup target list --installed | grep x86_64-unknown-none || echo "WARNING: target missing!"

echo "→ Launching FULL clean build with forced target..."
CARGO_BUILD_TARGET=x86_64-unknown-none ./scripts/clean_build.sh && CARGO_BUILD_TARGET=x86_64-unknown-none make run-sasos

echo "=========================================="
echo "SEX Microkernel v6 automation complete."
echo "cargo clean + env-forced target = core crate error should be DEAD."
