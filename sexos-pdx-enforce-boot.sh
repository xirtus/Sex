#!/bin/bash
set -euo pipefail
PROJECT_ROOT="/home/xirtus_arch/Documents/microkernel"
cd "$PROJECT_ROOT"

# Purge
cargo clean --quiet
rm -rf iso_root/ *.iso 2>/dev/null
mkdir -p iso_root/servers/

# Enforce Config
cat > limine.cfg << 'CFG_EOF'
TIMEOUT=1
:SexOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///sexos-kernel
    MODULE_PATH=boot:///servers/sexdisplay
    VIDEO_MODE=1280x720,32
CFG_EOF
cp limine.cfg iso_root/

# Rebuild
bash build_payload.sh
make clean
make iso

# Inspect
echo "--- INSPECTING ISO CONTENTS ---"
if command -v 7z >/dev/null 2>&1; then
    7z l sexos-v*.iso | grep -E 'limine.cfg|sexos-kernel|sexdisplay'
fi

make run-sasos
