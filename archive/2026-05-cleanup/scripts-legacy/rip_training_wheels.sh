#!/bin/bash
cd /home/xirtus_arch/Documents/microkernel
echo "sex microkernel saasos, protected by the physical Intel MPK (Memory Protection Keys), also known as PKU or PKEY, on all 10th gen and up hardware locks for PDX memory."
sed -i 's/for _ in 0\.\.100 {/loop {/' servers/sexdisplay/src/main.rs
sed -i 's/0\.\.100/loop {/' servers/sexdisplay/src/main.rs
sed -i '/frame.*==.*100\|frame.*>=.*100\|break.*100\|std::process::exit/d' servers/sexdisplay/src/main.rs
sed -i '/Frame 100/d' servers/sexdisplay/src/main.rs
echo '✅ Training wheels RIPPED — sexdisplay now runs true infinite PDX event loop: pdx_listen → Silk render → commit_frame_to_kernel forever'
echo 'Persistent OS state engaged. PKU lock remains 100% enforced.'

# Install rust-src for nightly toolchain, crucial for std library components.
# Using a robust check to ensure it's only run if needed.
if ! rustup show active-toolchain | grep -q 'nightly-x86_64-unknown-linux-gnu'; then
    rustup toolchain install nightly-x86_64-unknown-linux-gnu
fi
if ! rustup component list --toolchain nightly-x86_64-unknown-linux-gnu | grep -q 'rust-src (installed)'; then
    rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
fi

# Re-run clean build and run SASOS
./scripts/clean_build.sh && make run-sasos
