#!/bin/bash
set -e

echo "=========================================="
echo "  SEX MICROKERNEL - V8 AUTOMATION FIX"
echo "  https://github.com/xirtus/sex"
echo "=========================================="

echo "→ Forcing nightly toolchain + x86_64-unknown-none target..."
rustup toolchain install nightly --force
rustup target add x86_64-unknown-none --toolchain nightly
rustup component add rust-src --toolchain nightly

echo "→ Creating rust-toolchain.toml (locks nightly everywhere)..."
cat > rust-toolchain.toml << 'EOF'
[toolchain]
channel = "nightly"
components = ["rust-src"]
targets = ["x86_64-unknown-none"]
EOF

echo "→ Creating robust .cargo/config.toml..."
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[build]
target = "x86_64-unknown-none"
rustflags = ["-C", "target-cpu=generic"]

[unstable]
build-std = ["core", "alloc", "compiler_builtins"]
EOF

echo "→ Deep clean of ALL artifacts..."
cargo clean
rm -rf target/ 2>/dev/null || true

echo "→ Running cargo fix with explicit rustup + target..."
rustup run nightly cargo fix --allow-dirty --target x86_64-unknown-none -p sex-orbclient --lib || true
rustup run nightly cargo fix --allow-dirty --target x86_64-unknown-none -p tuxedo --lib || true
rustup run nightly cargo fix --allow-dirty --target x86_64-unknown-none -p sexgemini --bin sexgemini || true
rustup run nightly cargo fix --allow-dirty --target x86_64-unknown-none -p sexfiles || true

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

echo "→ Launching FULL clean build with rustup nightly + target..."
CARGO_BUILD_TARGET=x86_64-unknown-none rustup run nightly ./scripts/clean_build.sh && \
CARGO_BUILD_TARGET=x86_64-unknown-none rustup run nightly make run-sasos

echo "=========================================="
echo "SEX Microkernel v8 automation complete."
echo "rustup run nightly + explicit target = core crate error is DEAD."
