#!/usr/bin/env fish

echo "✦ SexOS SASOS: Cargo Workspace Profile Auto-Fixer"

set workspace_root "/home/xirtus_arch/Documents/microkernel"
set root_toml "$workspace_root/Cargo.toml"

# List of inner crates throwing the warnings
set target_crates \
    "$workspace_root/apps/linen/Cargo.toml" \
    "$workspace_root/servers/silk-shell/Cargo.toml" \
    "$workspace_root/servers/sexdisplay/Cargo.toml" \
    "$workspace_root/crates/tatami/Cargo.toml" \
    "$workspace_root/crates/silknet/Cargo.toml"

echo "→ Cleaning useless profile blocks from inner crates..."

for file in $target_crates
    if test -f $file
        # 1. Create a safe backup
        cp $file "$file.bak"
        
        # 2. Safely strip [profile.*] blocks until the next blank line or end of file
        # Using perl for multi-line regex safety without breaking other TOML blocks
        perl -0777 -pi -e 's/\[profile\.[^\]]+\]\s*(?:[a-zA-Z0-9_-]+\s*=\s*[^\n]+\s*)*//g' $file
        
        echo "  ✓ Cleaned: "(basename (dirname $file))"/Cargo.toml (Backup: .bak)"
    else
        echo "  ! Skipped: $file not found"
    end
end

echo "→ Checking Workspace Root for global release optimizations..."

# 3. Ensure the workspace root actually has a release profile so you don't lose optimizations
if not grep -q '\[profile.release\]' $root_toml
    echo "  ! No root release profile found. Appending standard bare-metal optimizations..."
    echo "" >> $root_toml
    echo "[profile.release]" >> $root_toml
    echo "opt-level = 3" >> $root_toml
    echo "lto = true" >> $root_toml
    echo "panic = 'abort'" >> $root_toml
    echo "  ✓ Added [profile.release] to root Cargo.toml"
else
    echo "  ✓ Workspace root already handles [profile.release]. Safe."
end

echo "✦ Fix Complete! Run ./build_payload.sh and the warnings will be gone."
