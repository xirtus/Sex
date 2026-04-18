#!/bin/bash
echo "--- Forcing no_std on all dependencies ---"

# Find every Cargo.toml and replace standard serde declarations with no_std versions
# This handles both: serde = "1.0"  AND  serde = { version = "1.0", features = [...] }
find . -name "Cargo.toml" -exec perl -i -pe 's/serde\s*=\s*"([^"]+)"/serde = { version = "$1", default-features = false, features = ["derive"] }/g' {} +
find . -name "Cargo.toml" -exec perl -i -pe 's/serde\s*=\s*\{ version\s*=\s*"([^"]+)"\s*\}/serde = { version = "$1", default-features = false, features = ["derive"] }/g' {} +

# Specific fix for bitflags which can also pull in std
find . -name "Cargo.toml" -exec perl -i -pe 's/bitflags\s*=\s*"([^"]+)"/bitflags = { version = "$1", default-features = false }/g' {} +

echo "--- Cleaning stale lockfile ---"
rm -f Cargo.lock
echo "Done. Try running ./run_sex.sh now."
