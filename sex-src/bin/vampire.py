#!/usr/bin/env python3
import os
import sys
import argparse

# vampire: The SexOS Autonomous Ingestion Engine
# "Vampires" mature code from upstream kernels (Linux/BSD) 
# and transforms it into isolated SexOS Protection Domains.
PROGNAME = "vampire"
LINUX_MIRROR = "/Users/xirtus/sites/Microkernel/upstream-linux"
MESA_MIRROR = "/Users/xirtus/sites/Microkernel/upstream-mesa"
WLROOTS_MIRROR = "/Users/xirtus/sites/Microkernel/upstream-wlroots"
SEX_PACKAGES = "/Users/xirtus/sites/Microkernel/sex-packages/srcpkgs"

def get_mirror(upstream_path):
    if upstream_path.startswith("mesa"):
        return MESA_MIRROR, upstream_path.replace("mesa/", "")
    if upstream_path.startswith("wlroots"):
        return WLROOTS_MIRROR, upstream_path.replace("wlroots/", "")
    return LINUX_MIRROR, upstream_path

def bite(upstream_path, pkg_name=None):
    mirror, path = get_mirror(upstream_path)
    if not pkg_name:
        pkg_name = os.path.basename(path).replace(".c", "") + "-sexdrive"

    print(f"[{PROGNAME}] Biting upstream source from mirror: {path} -> {pkg_name}")

    # 1. Trigger the AI Lifting Pipeline (passing mirror context)
    cmd = f"python3 /Users/xirtus/sites/Microkernel/sex-src/bin/sex-lift-ai.py {upstream_path}"
    os.system(cmd)

    # ... rest of bite logic ...


    # 2. Sync to Monorepo
    driver_name = os.path.basename(upstream_path).replace(".c", "")
    lift_dir = f"lifted_drivers/{driver_name}"
    pkg_dir = f"{SEX_PACKAGES}/{pkg_name}"
    os.makedirs(pkg_dir, exist_ok=True)
    
    if os.path.exists(f"{lift_dir}/template"):
        os.system(f"cp {lift_dir}/template {pkg_dir}/template")
        print(f"[{PROGNAME}] Sucked metadata into monorepo: {pkg_name}")
    
    return pkg_name

def infect(upstream_path):
    mirror, path = get_mirror(upstream_path)
    print(f"[{PROGNAME}] Infecting {path} from mirror with DDE-Sex shims...")
    
    driver_name = os.path.basename(path).replace(".c", "")
    lift_dir = f"lifted_drivers/{driver_name}"
    full_upstream = os.path.join(mirror, path)

    if not os.path.exists(full_upstream):
        print(f"[{PROGNAME}] Error: Upstream source not found at {full_upstream}")
        return

    # 1. Create source directory in lift_dir
    src_dir = f"{lift_dir}/src"
    os.makedirs(src_dir, exist_ok=True)

    # 2. Copy upstream source
    if os.path.isdir(full_upstream):
        os.system(f"cp -r {full_upstream}/* {src_dir}/")
    else:
        os.system(f"cp {full_upstream} {src_dir}/")

    # 3. Inject DDE-Sex header into all .c and .h files
    for root, _, files in os.walk(src_dir):
        for file in files:
            if file.endswith(".c") or file.endswith(".h"):
                file_path = os.path.join(root, file)
                try:
                    with open(file_path, "r") as f:
                        content = f.read()
                    with open(file_path, "w") as f:
                        f.write("#include <dde/sex_shim.h>\n" + content)
                except Exception as e:
                    print(f"[{PROGNAME}] Warning: Could not infect {file_path}: {e}")

    print(f"[{PROGNAME}] SUCCESS. {driver_name} is now INFECTED and ready for build.")

def main():
    parser = argparse.ArgumentParser(description="The SexOS Autonomous Ingestion Engine")
    parser.add_argument("action", choices=["bite", "drain", "infect"], help="vampire action")
    parser.add_argument("path", help="Upstream path to bite")
    args = parser.parse_args()

    if args.action == "bite":
        bite(args.path)
    elif args.action == "infect":
        infect(args.path)
    elif args.action == "drain":
        print(f"[{PROGNAME}] Draining binaries from {args.path}...")
    else:
        print(f"[{PROGNAME}] Unknown action.")

if __name__ == "__main__":
    main()
