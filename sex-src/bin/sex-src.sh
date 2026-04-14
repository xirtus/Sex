#!/usr/bin/env bash
# sex-src: Cross-compilation build tool for SexOS
# Inspired by xbps-src (Void Linux)

readonly PROGNAME=$(basename "$0")
readonly MASTERDIR="masterdir"
readonly DISTDIR=$(pwd)
readonly REPO_DIR="/Users/xirtus/sites/Microkernel/sex-packages"

usage() {
    echo "Usage: $PROGNAME pkg <package_name>"
    exit 1
}

if [ "$#" -lt 2 ]; then
    usage
fi

COMMAND="$1"
PKG="$2"

case "$COMMAND" in
    pkg)
        TEMPLATE="$REPO_DIR/srcpkgs/$PKG/template"
        if [ ! -f "$TEMPLATE" ]; then
            # Check local templates as fallback
            TEMPLATE="templates/$PKG/template"
            if [ ! -f "$TEMPLATE" ]; then
                echo "Error: Template for $PKG not found in $REPO_DIR or local templates."
                exit 1
            fi
        fi

        # 1. Source the template
        echo "sex-src: Building package '$PKG'..."
        source "$TEMPLATE"

        # 2. Handle Build Dependencies (Recursive call simulation)
        if [ -n "$makedepends" ]; then
            for dep in $makedepends; do
                echo "sex-src: Dependency '$dep' required for '$PKG'. Ensuring it's built..."
                # In a real system, we'd check if dep is already in binpkgs
                # $0 pkg "$dep"
            done
        fi

        # 2. Setup build directory
        WRKDIR="build_dir/$PKG-$version"
        mkdir -p "$WRKDIR"
        cd "$WRKDIR" || exit 1

        # 3. Download source (Mock)
        echo "sex-src: Fetching $distfiles..."
        # curl -L "$distfiles" -o source.tar.gz

        # 4. Run Build Step
        echo "sex-src: Running do_build..."
        if [ "$(type -t do_build)" = "function" ]; then
            do_build
        else
            echo "Warning: no do_build function found in template."
        fi

        # 5. Run Install Step (to a fake destdir)
        DESTDIR="$DISTDIR/destdir/$PKG-$version"
        mkdir -p "$DESTDIR"
        echo "sex-src: Running do_install..."
        if [ "$(type -t do_install)" = "function" ]; then
            do_install "$DESTDIR"
        else
            echo "Warning: no do_install function found in template."
        fi

        # 6. Generate .spd archive (Mock)
        echo "sex-src: Generating $PKG-$version.spd..."
        tar -czf "$DISTDIR/binpkgs/$PKG-$version.spd" -C "$DESTDIR" .
        echo "sex-src: SUCCESS. Package saved to binpkgs/$PKG-$version.spd"
        ;;
    *)
        usage
        ;;
esac
