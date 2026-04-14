#!/usr/bin/env bash
# sexpac: CLI Package Manager for SexOS
# Simulated installer for .spd (Sex Package Domain) archives

readonly PROGNAME=$(basename "$0")
readonly TARGET_ROOT="root_fs"

usage() {
    echo "Usage: $PROGNAME install <package_file.spd>"
    exit 1
}

if [ "$#" -lt 2 ]; then
    usage
fi

COMMAND="$1"
PKG_FILE="$2"

case "$COMMAND" in
    install)
        if [ ! -f "$PKG_FILE" ]; then
            echo "Error: Package file '$PKG_FILE' not found."
            exit 1
        fi

        PKG_NAME=$(basename "$PKG_FILE" .spd)
        echo "sexpac: Installing $PKG_NAME to $TARGET_ROOT..."

        # 1. Create target root
        mkdir -p "$TARGET_ROOT"

        # 2. Extract SPD (Zero-Copy extraction simulation)
        tar -xzf "$PKG_FILE" -C "$TARGET_ROOT"
        
        # 3. Process Manifest (Capabilities)
        if [ -f "$TARGET_ROOT/manifest.txt" ]; then
            echo "sexpac: WARNING: Package requested the following Capabilities:"
            cat "$TARGET_ROOT/manifest.txt" | grep "CAPABILITIES"
            echo "sexpac: Capabilities registered in sexvfs registry."
        fi

        echo "sexpac: SUCCESS. $PKG_NAME is now active in the SexOS SASOS."
        ;;
    *)
        usage
        ;;
esac
