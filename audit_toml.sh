#!/bin/bash
echo "--- Auditing Cargo.toml for no_std compliance ---"

# Find all Cargo.toml files and ensure default-features are off for common offenders
# This uses perl to safely edit the files across macOS/Linux
find . -name "Cargo.toml" -exec perl -i -pe 's/serde = \{ version = "([^"]+)" \}/serde = { version = "$1", default-features = false, features = ["derive"] }/g' {} +
find . -name "Cargo.toml" -exec perl -i -pe 's/bitflags = "([^"]+)"/bitflags = { version = "$1", default-features = false }/g' {} +

# Add a Workspace-level check to force no_std if using certain crates
# (Specifically addressing the serde_core issue)
