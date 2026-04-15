#!/usr/bin/env python3
import os
import sys
import json
import tarfile
import argparse

# sexpac: Advanced Package Manager for SexOS
# Manages .spd (Sex Package Domain) images and system registries.

TARGET_ROOT = "root_fs"
REGISTRY = "sexdrives.json"

class SexPac:
    def __init__(self):
        self.ensure_dirs()

    def ensure_dirs(self):
        if not os.path.exists(TARGET_ROOT):
            os.makedirs(TARGET_ROOT)

    def install(self, pkg_file):
        if not os.path.exists(pkg_file):
            print(f"Error: Package {pkg_file} not found.")
            return

        pkg_name = os.path.basename(pkg_file).replace(".spd", "")
        print(f"sexpac: Installing {pkg_name}...")

        # 1. Extract the SPD image
        with tarfile.open(pkg_file, "r:gz") as tar:
            tar.extractall(path=TARGET_ROOT)

        # 2. Parse manifest
        manifest_path = os.path.join(TARGET_ROOT, "manifest.json")
        if os.path.exists(manifest_path):
            with open(manifest_path, 'r') as f:
                manifest = json.load(f)
                self.process_capabilities(manifest.get("capabilities", []))
                self.register_driver(manifest.get("driver_info", {}))

        print(f"sexpac: SUCCESS. {pkg_name} installed successfully.")

    def process_capabilities(self, caps):
        if not caps: return
        print("sexpac: Registering Capabilities:")
        for cap in caps:
            print(f"  - {cap}")

    def register_driver(self, info):
        if not info: return
        print(f"sexpac: Registering hardware driver: {info.get('name')}")
        # In a real system, update sexdrives.json or call sexvfs PDX
        
    def build(self, src_dir, out_file):
        print(f"sexpac: Building SPD image from {src_dir} -> {out_file}...")
        with tarfile.open(out_file, "w:gz") as tar:
            tar.add(src_dir, arcname=".")
        print("sexpac: Build complete.")

def main():
    parser = argparse.ArgumentParser(description="SexOS Package Manager")
    subparsers = parser.add_subparsers(dest="command")

    # Install command
    install_parser = subparsers.add_parser("install", help="Install a .spd package")
    install_parser.add_argument("pkg_file", help="Path to the .spd file")

    # Build command
    build_parser = subparsers.add_parser("build", help="Build a .spd package from a directory")
    build_parser.add_argument("src_dir", help="Source directory")
    build_parser.add_argument("out_file", help="Output .spd file")

    args = parser.parse_args()
    pac = SexPac()

    if args.command == "install":
        pac.install(args.pkg_file)
    elif args.command == "build":
        pac.build(args.src_dir, args.out_file)
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
