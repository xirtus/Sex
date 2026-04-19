#!/bin/bash
set -e

echo "=========================================="
echo "  SEX MICROKERNEL - V7 AUTOMATION FIX"
echo "  https://github.com/xirtus/sex"
echo "=========================================="

echo "→ Creating rust-toolchain.toml (forces nightly + target everywhere)..."
cat > rust-toolchain.toml << 'EOF'
[toolchain]
channel = "nightly"
components = ["rust-src", "rustfmt", "clippy"]
targets = ["x86_64-unknown-none"]
EOF

echo "→ Updating .cargo/config.toml with build-std..."
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[build]
target = "x86_64-unknown-none"
rustflags = ["-C", "target-cpu=generic"]

[unstable]
build-std = ["core", "alloc", "compiler_builtins"]
EOF

echo "→ Deep cleaning ALL artifacts..."
cargo clean
rm -rf target/ ~/.cargo/registry/index/* ~/.cargo/registry/cache/* 2>/dev/null || true

echo "→ Installing everything on nightly..."
rustup toolchain install nightly --force
rustup target add x86_64-unknown-none --toolchain nightly
rustup component add rust-src --toolchain nightly
rustup override set nightly

echo "→ Explicit cargo fix with target..."
cargo +nightly fix --allow-dirty --target x86_64-unknown-none -p sex-orbclient --lib || true
cargo +nightly fix --allow-dirty --target x86_64-unknown-none -p tuxedo --lib || true
cargo +nightly fix --allow-dirty --target x86_64-unknown-none -p sexgemini --bin sexgemini || true
cargo +nightly fix --allow-dirty --target x86_64-unknown-none -p sexfiles || true

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

echo "→ Launching FULL clean build with explicit target..."
CARGO_BUILD_TARGET=x86_64-unknown-none ./scripts/clean_build.sh && CARGO_BUILD_TARGET=x86_64-unknown-none make run-sasos

echo "=========================================="
echo "SEX Microkernel v7 automation complete."
echo "rust-toolchain.toml + build-std + explicit --target = core crate error should be DEAD."
