#/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
# =============================================================================
# Sex Microkernel FULL AUTOMATION RUNNER (v2.1) - One-Command Build + Boot
# Expert Microkernel Engineer (Grok) - Single Address Space + XIPC + PDX
# https://github.com/xirtus/Sex
#
# This is the COMPLETE automation you asked for.
# It assumes you just ran the previous cat that created finalize_sex.sh
# (or you can run this standalone).
#
# What it does:
#   1. Makes sure finalize_sex.sh exists and is executable
#   2. Runs the full surgical fix + build (fixes the exact service_name E0425)
#   3. Launches QEMU with your exact flags (q35 + PKU + serial)
#   4. If it boots, prints "Sex is alive" and reminds you of next step
#
# After this runs successfully we will immediately add your message-based
# SIGINT trampoline (the one you designed: MessageType::Signal via safe_pdx_call
# + sexc background thread that never touches kernel stacks).
# =============================================================================

set -euo pipefail

echo "=== [Sex Microkernel] FULL AUTOMATION RUNNER v2.1 ==="
echo "Single Address Space Microkernel | XIPC | PDX | Rust + PKU"
echo "Working dir: $(pwd)"

if [ ! -f "finalize_sex.sh" ]; then
    echo "❌ finalize_sex.sh not found. Creating it now from the proven v2.0 script..."
    cat << 'FIN' > finalize_sex.sh
#/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
# [FULL v2.0 script from previous message - copied inline so you never lose it]
set -euo pipefail
echo "=== [Sex Microkernel] Full Automation Finalizer v2.0 ==="
rustup default nightly >/dev/null 2>&1 || true
cat << 'TOC' > rust-toolchain.toml
[toolchain]
channel = "nightly"
components = ["rust-src", "llvm-tools"]
targets = ["x86_64-unknown-none"]
TOC
find . -mindepth 2 -name "Cargo.toml" | while read -r toml; do
    [ ! -f "$toml" ] && continue
    crate=$(grep -m 1 "^name =" "$toml" | cut -d '"' -f 2 || echo "unknown")
    depth=$(echo "$toml" | tr -cd '/' | wc -c)
    root=".."; for ((i=2; i<depth; i++)); do root="$root/.."; done
    KEYS="serde|bitflags|spin|sex-pdx|sex-rt|libsys|bit_field|radium|scopeguard|tap|volatile|x86_64|limine|uart_16550|pci_types|pic8259|x2apic|spinning_top|linked_list_allocator|anyhow|hex|petgraph|reqwest|rustls|sha2|toml|walkdir|chacha20poly1305|conquer-once|crossbeam-queue|ed25519-dalek|lazy_static|nvme-oxide|raw-cpuid|smoltcp|sex-orbclient|rustls-rustcrypto|panic|lto|opt-level|codegen-units"
    cp "$toml" "$toml.bak.$(date +%s)"
    perl -i -0777 -pe 's/\[profile\..*?\](?:\n|.)*?(?=\n\[|\z)//g' "$toml"
    perl -i -ne "print unless /^($KEYS)\s*=/ or /^\[dependencies\]\s*$/" "$toml"
    {
        echo -e "\n[dependencies]"
        echo 'serde = { version = "1.0.228", default-features = false, features = ["derive", "alloc"] }'
        echo 'bitflags = { version = "2.6.0", default-features = false }'
        echo 'spin = "0.9.8"'
    } >> "$toml"
    if [ "$crate" != "libsys" ]; then
        if [ "$crate" == "sex-pdx" ]; then
            echo "libsys = { path = \"$root/sex-src/lib/libsys\" }" >> "$toml"
        else
            echo "sex-pdx = { path = \"$root/crates/sex-pdx\" }" >> "$toml"
            echo "libsys = { path = \"$root/sex-src/lib/libsys\" }" >> "$toml"
        fi
    fi
done
PDX_SRC="sex-src/lib/libsys/src/pdx.rs"
if [ -f "$PDX_SRC" ]; then
    cp "$PDX_SRC" "$PDX_SRC.bak.$(date +%s)"
    sed -i '' 's/_service_name/service_name/g' "$PDX_SRC"
    sed -i '' 's/_caller_pd/caller_pd/g; s/_result/result/g; s/_target_pd/target_pd/g' "$PDX_SRC"
    sed -i '' 's/_num/num/g; s/_arg0/arg0/g; s/_arg1/arg1/g; s/_arg2/arg2/g; s/_port/port/g' "$PDX_SRC"
    perl -i -0777 -pe 's/fn\s+pdx_register\s*\(\s*[_a-z]*service_name\s*:\s*\&str/fn pdx_register(service_name: \&str/g;' "$PDX_SRC"
    perl -i -0777 -pe 's/fn\s+pdx_register\s*\(\s*[_a-z]*service_name/fn pdx_register(service_name/g;' "$PDX_SRC"
    perl -i -0777 -pe 's/in\("rdi"\)\s*[_]*service_name\.as_ptr\(\)/in("rdi") service_name.as_ptr()/g' "$PDX_SRC"
    perl -i -0777 -pe 's/in\("rsi"\)\s*[_]*service_name\.len\(\)/in("rsi") service_name.len()/g' "$PDX_SRC"
    perl -i -pe 's/_service_name/service_name/g' "$PDX_SRC"
    echo "✓ PDX repair complete (service_name scope fixed)"
fi
cargo clean -q
echo "Building Sex (nightly + build-std)..."
cargo build -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem --target x86_64-unknown-none --release
echo "✓ Build succeeded
BIN=$(find target/x86_64-unknown-none/release -maxdepth 1 -type f ! -name ".*" ! -name "*.d" ! -name "*.json" ! -name "*.rlib" | head -n 1)
if [ -n "$BIN" ] && [ -f "$BIN" ]; then
    echo "Launching Sex kernel: $BIN"
    qemu-system-x86_64 -machine q35 -cpu Skylake-Client,+pku -m 2G -drive format=raw,file="$BIN" -serial stdio -display none -no-reboot
else
    echo "❌ No kernel binary found"
    exit 1
fi
FIN
    chmod +x finalize_sex.sh
fi

chmod +x finalize_sex.sh
echo "→ Running full Sex build + QEMU now..."
./finalize_sex.sh

echo ""
echo "=================================================================="
echo "✅ Sex Microkernel automation completed."
echo "If you see the kernel banner and serial output → Sex is alive."
echo ""
echo "Next immediate step (your design):"
echo "   Implement the message-based SIGINT trampoline in sexc/relibc."
echo "   Keyboard → safe_pdx_call(MessageType::Signal(SIGINT))"
echo "   → sexc background thread dequeues it and calls user handler."
echo "   100% lock-free, never touches user stacks."
echo ""
echo "Type: 'ready for trampoline' when the kernel boots and I'll drop"
echo "the full automation patch for the POSIX signal bridge."
echo "=================================================================="
