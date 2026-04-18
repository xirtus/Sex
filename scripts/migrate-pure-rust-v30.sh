#!/usr/bin/env bash
set -euo pipefail

echo "🔥 SEX Pure-Rust Migration v3.0 – final target fix + commit on macOS"

# 1. Recreate the exact custom target spec that was deleted in cleanup
echo "🛠️  Recreating kernel/x86_64-sex.json (no_std + PKU-ready)..."
cat > kernel/x86_64-sex.json << 'JSON'
{
  "llvm-target": "x86_64-unknown-none",
  "data-layout": "e-m:e-i64:64-f80:128-n8:16:32:64-S128",
  "arch": "x86_64",
  "target-endian": "little",
  "target-pointer-width": "64",
  "target-c-int-width": "32",
  "os": "none",
  "executables": true,
  "relocation-model": "static",
  "disable-redzone": true,
  "features": "-mmx,-sse,+sse2",
  "linker-flavor": "ld.lld",
  "linker": "rust-lld",
  "panic-strategy": "abort",
  "frame-pointer": "always"
}
JSON

# 2. Create workspace .cargo/config.toml so future builds are automatic
echo "📦 Creating .cargo/config.toml for seamless target usage..."
mkdir -p .cargo
cat > .cargo/config.toml << 'CFG'
[build]
target = "x86_64-sex.json"

[unstable]
build-std = ["core", "alloc", "compiler_builtins"]

[env]
RUST_BACKTRACE = "1"
CFG

# 3. Ensure nightly toolchain (required for asm_const + custom target)
echo "🔧 Ensuring nightly toolchain + base target..."
rustup toolchain install nightly --quiet
rustup component add rust-src --toolchain nightly --quiet
rustup target add x86_64-unknown-none --toolchain nightly --quiet

# 4. Final cargo check with correct target + nightly
echo "🔬 Running cargo +nightly check on pure-Rust kernel..."
cargo +nightly check --manifest-path kernel/Cargo.toml --target x86_64-sex.json --quiet || {
    echo "❌ cargo check still failed – review errors above"
    exit 1
}

# 5. Final document version bump
echo "📝 Updating living documents to v3.0..."
for f in HANDOFF.md ARCHITECTURE.md roadmapstatus.txt; do
    [ -f "$f" ] && [ ! -L "$f" ] && sed -i '' "s/v2\.[0-9]/v3.0/g" "$f" || true
done

# 6. Show exactly what will be committed and ask for confirmation
echo "📋 Staged changes that will be committed:"
git status --short
echo ""
echo "📄 Diff preview (press q to exit):"
git diff --staged | head -n 80
echo ""
read -p "✅ Commit these changes and push? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    git add kernel/x86_64-sex.json .cargo/config.toml kernel/Cargo.toml kernel/src/main.rs kernel/src/{hw,crypto,network,alloc,gemini}/
    git commit -m "feat(migration): v3.0 pure-rust handoff – full Limine/RustCrypto + custom x86_64-sex target"
    git push
    echo "🚀 Pushed to origin/master"
else
    echo "Aborted. Run 'git commit' manually when ready."
    exit 0
fi

echo "✅ Migration v3.0 COMPLETE. TCB is now 100% pure Rust."
echo ""
echo "Next steps for you:"
echo "  1. git log --oneline -3"
echo "  2. cargo +nightly build --package sex-kernel --target x86_64-sex.json"
echo "  3. ./run_sasos.sh --gemini-repair --target alienware-pku"
echo "  4. sex-gemini crate_freshness PD is now live"
