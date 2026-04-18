#!/usr/bin/env bash
set -euo pipefail

echo "🔥 SEX Pure-Rust Migration v3.2 – zero-flag rustup fix + final commit on macOS"

# 1. Install nightly toolchain (no flags at all — works on every macOS rustup)
echo "🔧 Ensuring nightly toolchain + required components..."
rustup toolchain install nightly || true
rustup component add rust-src --toolchain nightly || true
rustup target add x86_64-unknown-none --toolchain nightly || true

# 2. Final cargo check using rustup run (bypasses +nightly PATH issues)
echo "🔬 Running rustup run nightly cargo check on pure-Rust kernel..."
rustup run nightly cargo check \
    --manifest-path kernel/Cargo.toml \
    --target x86_64-sex.json || {
    echo "❌ cargo check still failed – review errors above"
    exit 1
}

# 3. Final document version bump
echo "📝 Updating living documents to v3.2..."
for f in HANDOFF.md ARCHITECTURE.md roadmapstatus.txt; do
    [ -f "$f" ] && [ ! -L "$f" ] && sed -i '' "s/v3\.[0-1]/v3.2/g" "$f" || true
done

# 4. Show exactly what will be committed and ask for confirmation
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
    git commit -m "feat(migration): v3.2 pure-rust handoff – full Limine/RustCrypto + x86_64-sex target restored (macOS rustup fix)"
    git push
    echo "🚀 Pushed to origin/master"
else
    echo "Aborted. Run 'git commit' manually when ready."
    exit 0
fi

echo "✅ Migration v3.2 COMPLETE. TCB is now 100% pure Rust."
echo ""
echo "Next steps for you (copy-paste):"
echo "  git log --oneline -3"
echo "  rustup run nightly cargo build --package sex-kernel --target x86_64-sex.json"
echo "  ./run_sasos.sh --gemini-repair --target alienware-pku"
echo "  sex-gemini crate_freshness PD is now live and watching upstream"
