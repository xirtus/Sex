#!/bin/bash
# =============================================================================
# Sex Microkernel — DOCKER BUILD + MAC BOOT (M1/Apple Silicon)
# Target: x86_64-unknown-none
# =============================================================================

set -euo pipefail

echo "=== [Sex Microkernel] M1 DOCKER BUILD + BOOT ==="
echo "Working directory: $(pwd)"
echo ""

# ----------------------------------------------------------------------------
echo "--- [1/3] PDX Source Repair ---"
PDX_SRC="sex-src/lib/libsys/src/pdx.rs"

if [ ! -f "$PDX_SRC" ]; then
    echo "❌ $PDX_SRC not found. cd into the root of your Sex repo and try again."
    exit 1
fi

cp "$PDX_SRC" "$PDX_SRC.bak.$(date +%s)"
sed -i "" "s/_service_name/service_name/g" "$PDX_SRC"
sed -i "" "s/_caller_pd/caller_pd/g; s/_result/result/g; s/_target_pd/target_pd/g" "$PDX_SRC"
sed -i "" "s/_num/num/g; s/_arg0/arg0/g; s/_arg1/arg1/g; s/_arg2/arg2/g" "$PDX_SRC"

perl -i -0777 -pe "s/fn\s+pdx_register\s*\(\s*[_a-z]*service_name/fn pdx_register(service_name/g" "$PDX_SRC"
perl -i -0777 -pe "s/in\(\"rdi\"\)\s*[_]*service_name\.as_ptr\(\)/in(\"rdi\") service_name.as_ptr()/g" "$PDX_SRC"
perl -i -0777 -pe "s/in\(\"rsi\"\)\s*[_]*service_name\.len\(\)/in(\"rsi\") service_name.len()/g" "$PDX_SRC"
echo "✓ PDX repair complete"

# ----------------------------------------------------------------------------
echo "--- [2/3] Cargo.toml Normalization ---"
KEYS="serde|bitflags|spin|sex-pdx|sex-rt|libsys|bit_field|radium|scopeguard|tap|volatile|x86_64|limine|uart_16550|pci_types|pic8259|x2apic|spinning_top|linked_list_allocator|anyhow|hex|petgraph|reqwest|rustls|sha2|toml|walkdir|chacha20poly1305|conquer-once|crossbeam-queue|ed25519-dalek|lazy_static|nvme-oxide|raw-cpuid|smoltcp|sex-orbclient|rustls-rustcrypto|panic|lto|opt-level|codegen-units"

find . -mindepth 2 -name "Cargo.toml" | while read -r toml; do
    [ ! -f "$toml" ] && continue
    crate=$(grep -m 1 "^name =" "$toml" | cut -d '"' -f 2 || echo "unknown")
    
    cp "$toml" "$toml.bak.$(date +%s)" 2>/dev/null || true
    perl -i -0777 -pe "s/\[profile\..*?\](?:\n|.)*?(?=\n\[|\z)//g" "$toml"
    perl -i -ne "print unless /^($KEYS)\s*=/ or /^\[dependencies\]\s*$/" "$toml"

    {
        echo -e "\n[dependencies]"
        echo "serde = { version = \"1.0.228\", default-features = false, features = [\"derive\", \"alloc\"] }"
        echo "bitflags = { version = \"2.6.0\", default-features = false }"
        echo "spin = \"0.9.8\""
    } >> "$toml"

    if [ "$crate" != "libsys" ]; then
        if [ "$crate" == "sex-pdx" ]; then
            echo "libsys = { path = \"../sex-src/lib/libsys\" }" >> "$toml"
        else
            echo "sex-pdx = { path = \"../crates/sex-pdx\" }" >> "$toml"
            echo "libsys = { path = \"../sex-src/lib/libsys\" }" >> "$toml"
        fi
    fi
done
echo "✓ Manifests normalized"

# ----------------------------------------------------------------------------
echo "--- [3/3] Docker Build + QEMU Boot ---"
echo "Booting up x86_64 Rust Docker container via Rosetta..."

# Run the build inside an x86_64 linux container
docker run --rm \
    --platform linux/amd64 \
    -v "$(pwd):/work" \
    -w /work \
    rustlang/rust:nightly-bullseye \
    bash -c "
        set -e
        rustup component add rust-src llvm-tools-preview
        cargo clean -q
        echo 'Compiling...'
        cargo build \
            -Z build-std=core,alloc \
            -Z build-std-features=compiler-builtins-mem \
            --target x86_64-unknown-none \
            --release
    "

BIN=$(find target/x86_64-unknown-none/release -maxdepth 1 -type f ! -name ".*" ! -name "*.d" ! -name "*.json" ! -name "*.rlib" | head -n 1)

if [ -n "$BIN" ] && [ -f "$BIN" ]; then
    echo "✅ BUILD SUCCESS — launching Sex kernel"
    echo "   Binary: $BIN"
    echo ""
    echo "Booting in QEMU (Ctrl+C to exit)..."
    
    # We use -cpu qemu64 because Skylake-Client/+pku can cause issues on M1 hardware virtualization via QEMU
    qemu-system-x86_64 \
        -machine q35 \
        -cpu qemu64 \
        -m 2G \
        -drive format=raw,file="$BIN" \
        -serial stdio \
        -display none \
        -no-reboot
else
    echo "❌ Build failed. Check the Docker output above."
    exit 1
fi
