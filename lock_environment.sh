#!/usr/bin/env bash

# Exit immediately if a command fails
set -e

echo "[*] SECURING SASOS BUILD ENVIRONMENT..."

# 1. Permanently fix the Homebrew vs Rustup PATH conflict
# This ensures that every time you open a new terminal, the correct 'cargo' is used.
if ! grep -q 'export PATH="$HOME/.cargo/bin:$PATH"' ~/.zshrc; then
    echo ">>> Patching ~/.zshrc to prioritize Rustup..."
    echo "" >> ~/.zshrc
    echo "# SASOS Toolchain: Prioritize rustup over Homebrew" >> ~/.zshrc
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
else
    echo ">>> ~/.zshrc already contains the correct PATH settings."
fi

# 2. Force the current session to use the correct PATH before running rustup
export PATH="$HOME/.cargo/bin:$PATH"

# 3. Lock this specific directory to the nightly toolchain permanently
echo ">>> Enforcing nightly toolchain and required components for this directory..."
rustup override set nightly
rustup component add rust-src

echo "[*] ENVIRONMENT LOCKED SUCCESSFULLY."
echo "[!] IMPORTANT: Run 'source ~/.zshrc' right now to apply the PATH fix to this terminal."
echo "[!] From now on, you can open a fresh terminal and just run:"
echo "    ./scripts/clean_build.sh && make run-sasos"
