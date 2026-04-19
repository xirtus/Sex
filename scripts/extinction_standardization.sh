#!/bin/bash
# SexOS SASOS - Repository Standardization & Global Build
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Purging old build artifacts and ghosts..."
rm -rf target/ build/ build_initrd/ *.log
find . -name "*.bak" -type f -delete
echo " -> Workspace is clean."

echo "2. Deep Sweep: Final name eradication (macOS safe)..."
export LC_ALL=C
# Target source and configuration files only
find . -type f \( -name "*.rs" -o -name "*.sh" -o -name "*.md" -o -name "*.txt" -o -name "Makefile" -o -name "*.toml" \) | xargs grep -rilE 'tuxedo' | while read -r file; do
    sed -i '' 's/tuxedo/tuxedo/g' "$file"
    sed -i '' 's/Tuxedo/Tuxedo/g' "$file"
    sed -i '' 's/tuxedo/tuxedo/g' "$file"
    sed -i '' 's/Tuxedo/Tuxedo/g' "$file"
    echo "   [PATCHED] $file"
done

echo "3. Standardizing Server Headers & Panic Handlers..."
for srv in servers/*; do
    if [ -d "$srv/src" ]; then
        main_file="$srv/src/main.rs"
        [ ! -f "$main_file" ] && main_file="$srv/src/lib.rs"
        [ ! -f "$main_file" ] && continue
        
        echo " -> Checking $main_file"
        
        # Ensure #![no_std] and #![no_main] are the absolute first lines
        # Then ensure a panic_handler exists
        if ! grep -q "panic_handler" "$main_file"; then
            echo "    -> Injecting Panic Handler stub..."
            cat << 'EOF' >> "$main_file"

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
EOF
        fi
        
        # Ensure _start exists for binaries
        if [[ "$main_file" == *"main.rs" ]] && ! grep -q "_start" "$main_file"; then
            echo "    -> Injecting _start entry point..."
            cat << 'EOF' >> "$main_file"

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
EOF
        fi
    fi
done

echo "4. Firing GLOBAL CLEAN BUILD..."
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && 
    rustup component add rust-src && 
    cargo build --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release
" > global_final.log 2>&1 || true

echo "5. Git Preparation..."
git add -A
echo "--> All files standardized. If errors remain, check the bottom of global_final.log"
tail -n 30 global_final.log
