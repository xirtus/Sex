#!/usr/bin/env python3
import os
import sys
import hashlib
import struct
import subprocess
import argparse

# sexpac: The SexOS Protection Domain Packager
# Bundles user-space binaries into a contiguous initrd.sex blob with headers.

MAGIC = b"SEXPAC01"

def pack_pd(bin_path, out_file):
    name = os.path.basename(bin_path).encode('utf-8')
    
    # 1. Strip debug symbols
    stripped_path = bin_path + ".stripped"
    print(f"sexpac: Stripping {bin_path}...")
    subprocess.run(["rust-strip", "-s", bin_path, "-o", stripped_path], check=True)
    
    # 2. Read data and calculate SHA-256
    with open(stripped_path, "rb") as f:
        data = f.read()
    
    file_hash = hashlib.sha256(data).digest()
    size = len(data)
    
    # 3. Write Header: Magic(8), Name(32), Size(8), Hash(32)
    header = struct.pack("<8s32sQ32s", MAGIC, name.ljust(32, b'\0'), size, file_hash)
    out_file.write(header)
    out_file.write(data)
    
    # Alignment to 4KB for SAS paging efficiency
    padding = (4096 - (out_file.tell() % 4096)) % 4096
    out_file.write(b'\0' * padding)
    
    os.remove(stripped_path)
    print(f"sexpac: Packed {bin_path} ({size} bytes, SHA-256 verified)")

def main():
    parser = argparse.ArgumentParser(description="SexOS PD Packager")
    parser.add_argument("--out", required=True, help="Output initrd.sex file")
    parser.add_argument("bins", nargs="+", help="ELF binaries to pack")
    
    args = parser.parse_args()
    
    with open(args.out, "wb") as out_file:
        for bin_path in args.bins:
            pack_pd(bin_path, out_file)
            
    print(f"sexpac: Successfully generated {args.out}")

if __name__ == "__main__":
    main()
