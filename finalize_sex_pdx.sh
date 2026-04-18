#!/usr/bin/env bash
set -e

PROJECT_DIR="/Users/xirtus/sites/microkernel"
CRATE_DIR="$PROJECT_DIR/crates/sex-pdx"
LIB_FILE="$CRATE_DIR/src/lib.rs"
CARGO_FILE="$CRATE_DIR/Cargo.toml"

echo "============================================================"
echo "[*] Phase 1: Fixing Cargo.toml Features"
echo "============================================================"

# Ensure the [features] section exists and includes serde
if ! grep -q "\[features\]" "$CARGO_FILE"; then
    echo "Creating [features] section..."
    cat >> "$CARGO_FILE" << 'EOT'

[features]
default = []
serde = ["dep:serde"]
EOT
elif ! grep -q "serde =" "$CARGO_FILE"; then
    echo "Adding serde feature to existing section..."
    sed -i '' '/\[features\]/a \
serde = ["dep:serde"]' "$CARGO_FILE"
fi

echo "============================================================"
echo "[*] Phase 2: Enforcing Mutability in lib.rs"
echo "============================================================"

# Change 'let req =' to 'let mut req =' specifically for the PdxRequest initialization
sed -i '' 's/let req = PdxRequest::default()/let mut req = PdxRequest::default()/g' "$LIB_FILE"

echo "[+] Source code patched. 'req' is now mutable."

echo "============================================================"
echo "[*] Phase 3: Final Build Execution"
echo "============================================================"

cd "$PROJECT_DIR"
./scripts/clean_build.sh

echo "============================================================"
echo "[+] DONE: ISO should now be generated successfully."
echo "============================================================"
